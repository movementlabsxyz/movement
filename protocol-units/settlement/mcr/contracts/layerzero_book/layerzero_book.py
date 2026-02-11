#!/usr/bin/env python3
"""
Generate LayerZero V2 Address Book Solidity contracts from metadata API.

This script fetches deployment data from LayerZero's metadata API and generates
Solidity library contracts following the Aave address book pattern.
"""

import json
import requests
from typing import Dict, Any, List, Optional
import re
from web3 import Web3
import hashlib
from datetime import datetime

# Constants
METADATA_URL = "https://metadata.layerzero-api.com/v1/metadata/deployments"
SOLIDITY_HEADER = """// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

// Auto-generated from LayerZero metadata - do not edit manually
// Source: https://metadata.layerzero-api.com/v1/metadata/deployments

import {ILayerZeroEndpointV2} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import {IMessageLib} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/IMessageLib.sol";
"""

# Map of contract names to their expected types
CONTRACT_TYPES = {
    "endpoint": "ILayerZeroEndpointV2",
    "endpointV2": "ILayerZeroEndpointV2",
    "sendUln301": "IMessageLib",
    "receiveUln301": "IMessageLib",
    "sendUln302": "IMessageLib",
    "receiveUln302": "IMessageLib",
    "readLib1002": "IMessageLib",
}

def to_checksum_address(address: str) -> str:
    """Convert address to EIP-55 checksum format."""
    # Skip if not a valid Ethereum address (must be 42 chars including 0x)
    if not address or len(address) != 42:
        return address
    try:
        return Web3.to_checksum_address(address)
    except ValueError:
        # If it's not a valid address, return as-is
        return address

def to_pascal_case(snake_str: str) -> str:
    """Convert snake_case or kebab-case to PascalCase."""
    # Replace hyphens with underscores for uniform processing
    snake_str = snake_str.replace('-', '_')
    # Split by underscore and capitalize each part
    parts = snake_str.split('_')
    return ''.join(word.capitalize() for word in parts)

def sanitize_chain_name(chain_key: str) -> str:
    """Sanitize chain key to valid Solidity identifier."""
    # Keep the full chain name to avoid duplicates
    # Just convert to PascalCase without removing suffixes
    return to_pascal_case(chain_key)

def sanitize_dvn_name(canonical_name: str) -> str:
    """Convert DVN canonical name to valid Solidity constant name."""
    # Handle special cases
    special_cases = {
        "LayerZero Labs": "LAYERZERO_LABS",
        "LZDeadDVN": "LZ_DEAD",
        "TSS": "TSS",
        "01node": "01NODE",
        "P-OPS Team": "P_OPS_TEAM",
        "Animoca Blockdaemon": "ABDB",
        "ABDB": "ABDB",
        "P2P": "P2P",
    }
    
    if canonical_name in special_cases:
        return special_cases[canonical_name]
    
    # General conversion: uppercase and replace spaces/special chars with underscores
    name = canonical_name.upper()
    name = re.sub(r'[^A-Z0-9]+', '_', name)
    name = name.strip('_')
    
    # Ensure it doesn't start with a number
    if name and name[0].isdigit():
        name = 'DVN_' + name
    
    return name

def generate_constant_name(key: str) -> str:
    """Generate a proper constant name from a camelCase key."""
    # Special mappings
    special_mappings = {
        "endpointV2": "ENDPOINT_V2",
        "endpointV2View": "ENDPOINT_V2_VIEW",
        "relayerV2": "RELAYER_V2",
        "ultraLightNodeV2": "ULTRA_LIGHT_NODE_V2",
        "sendUln301": "SEND_ULN_301",
        "receiveUln301": "RECEIVE_ULN_301",
        "sendUln302": "SEND_ULN_302",
        "receiveUln302": "RECEIVE_ULN_302",
        "readLib1002": "READ_LIB_1002",
        "lzExecutor": "LZ_EXECUTOR",
        "deadDVN": "DEAD_DVN",
        "blockedMessageLib": "BLOCKED_MESSAGE_LIB",
        "nonceContract": "NONCE_CONTRACT",
    }
    
    if key in special_mappings:
        return special_mappings[key]
    
    # Convert camelCase to UPPER_SNAKE_CASE
    result = re.sub('([a-z0-9])([A-Z])', r'\1_\2', key)
    return result.upper()

def is_valid_ethereum_address(address: str) -> bool:
    """Check if the string is a valid Ethereum address."""
    if not address or not isinstance(address, str):
        return False
    # Ethereum addresses are 42 chars (0x + 40 hex chars)
    if len(address) != 42 or not address.startswith('0x'):
        return False
    # Check if the rest are valid hex characters
    try:
        int(address[2:], 16)
        return True
    except ValueError:
        return False

def format_deployment_constant(key: str, address: str, contract_type: Optional[str] = None) -> str:
    """Format a deployment as a Solidity constant."""
    # Skip if not a valid Ethereum address
    if not is_valid_ethereum_address(address):
        return None
        
    const_name = generate_constant_name(key)
    checksum_addr = to_checksum_address(address)
    
    if contract_type:
        return f"  {contract_type} internal constant {const_name} = {contract_type}({checksum_addr});"
    else:
        return f"  address internal constant {const_name} = {checksum_addr};"

def generate_chain_library(chain_key: str, chain_data: Dict[str, Any]) -> Optional[str]:
    """Generate a Solidity library for a single chain."""
    deployments = chain_data.get('deployments', [])
    if not deployments:
        return None
    
    # Find V2 deployment (prefer version 2)
    # If multiple V2 deployments exist, prefer the one with higher EID (testnet)
    v2_deployment = None
    v1_deployment = None
    
    for deployment in deployments:
        if deployment.get('version') == 2:
            if v2_deployment is None:
                v2_deployment = deployment
            else:
                # If we have multiple V2 deployments, prefer the one with higher EID
                current_eid = deployment.get('eid', 0)
                existing_eid = v2_deployment.get('eid', 0)
                if current_eid > existing_eid:
                    v2_deployment = deployment
        elif deployment.get('version') == 1:
            v1_deployment = deployment
    
    # Use V2 if available, otherwise V1
    deployment = v2_deployment or v1_deployment
    if not deployment:
        return None
    
    chain_name = sanitize_chain_name(chain_key)
    library_name = f"LayerZeroV2{chain_name}"
    
    # Start building the library
    lines = [f"\nlibrary {library_name} {{"]
    
    # Add chain metadata
    eid = deployment.get('eid', '')
    chain_details = chain_data.get('chainDetails', {})
    native_chain_id = chain_details.get('nativeChainId', 0)
    
    if eid:
        lines.append(f"  // Chain metadata")
        lines.append(f"  uint32 internal constant EID = {eid};")
        if native_chain_id:
            lines.append(f"  uint256 internal constant CHAIN_ID = {native_chain_id};")
        lines.append(f'  string internal constant CHAIN_KEY = "{chain_key}";')
        lines.append("")
    
    # Group contracts by category
    core_contracts = []
    message_libs = []
    other_contracts = []
    
    # Process all contract addresses in the deployment
    for key, value in deployment.items():
        if key in ['eid', 'chainKey', 'stage', 'version']:
            continue
        
        if isinstance(value, dict) and 'address' in value:
            address = value['address']
            contract_type = CONTRACT_TYPES.get(key)
            
            constant_line = format_deployment_constant(key, address, contract_type)
            
            # Skip if not a valid address
            if constant_line is None:
                continue
            
            # Categorize the constant
            if key in ['endpoint', 'endpointV2']:
                core_contracts.insert(0, constant_line)  # Put endpoint first
            elif 'Uln' in key or 'Lib' in key:
                message_libs.append(constant_line)
            else:
                other_contracts.append(constant_line)
    
    # Add categorized constants
    if core_contracts:
        lines.append("  // Core protocol")
        lines.extend(core_contracts)
        if message_libs or other_contracts:
            lines.append("")
    
    if message_libs:
        lines.append("  // Message libraries")
        lines.extend(message_libs)
        if other_contracts:
            lines.append("")
    
    if other_contracts:
        lines.append("  // Other contracts")
        lines.extend(other_contracts)
    
    lines.append("}")
    
    return "\n".join(lines)

def generate_dvn_library(chain_key: str, dvns: Dict[str, Any]) -> Optional[str]:
    """Generate a DVN library for a chain."""
    if not dvns:
        return None
    
    chain_name = sanitize_chain_name(chain_key)
    library_name = f"LayerZeroV2DVN{chain_name}"
    
    lines = [f"\nlibrary {library_name} {{"]
    
    # Track used constant names to avoid duplicates
    used_names = set()
    
    # Sort DVNs by canonical name for consistent output
    sorted_dvns = sorted(dvns.items(), key=lambda x: x[1].get('canonicalName', ''))
    
    for address, dvn_info in sorted_dvns:
        canonical_name = dvn_info.get('canonicalName', '')
        dvn_id = dvn_info.get('id', '')
        deprecated = dvn_info.get('deprecated', False)
        
        if not canonical_name:
            continue
            
        # Skip if not a valid Ethereum address
        if not is_valid_ethereum_address(address):
            continue
        
        # Generate constant name
        base_name = f"DVN_{sanitize_dvn_name(canonical_name)}"
        const_name = base_name
        
        # Handle duplicates by adding suffix
        suffix = 2
        while const_name in used_names:
            const_name = f"{base_name}_{suffix}"
            suffix += 1
        
        used_names.add(const_name)
        checksum_addr = to_checksum_address(address)
        
        # Add comment with metadata
        comment_parts = [canonical_name]
        if deprecated:
            comment_parts.append("(deprecated)")
        if dvn_id:
            comment_parts.append(f"[{dvn_id}]")
        
        lines.append(f"  // {' '.join(comment_parts)}")
        lines.append(f"  address internal constant {const_name} = {checksum_addr};")
    
    lines.append("}")
    
    return "\n".join(lines)

def fetch_metadata() -> Dict[str, Any]:
    """Fetch metadata from LayerZero API."""
    print(f"Fetching metadata from {METADATA_URL}...")
    response = requests.get(METADATA_URL)
    response.raise_for_status()
    return response.json()

def generate_lz_protocol(chains_processed: List[str], metadata: Dict[str, Any]) -> str:
    """Generate the LZProtocol contract."""
    registry_header = """// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "../src-gen/LZAddresses.sol";
import "./interfaces/ILZProtocol.sol";

/// @title LayerZero Protocol Address Provider
/// @notice Provides LayerZero protocol addresses from the generated address book
/// @dev Implements ILZProtocol interface for clean access to core protocol addresses
contract LZProtocol is ILZProtocol {
    // Storage for chain data
    mapping(uint32 => ProtocolAddresses) private _protocolAddresses;
    
    // List of registered EIDs for iteration
    uint32[] private _registeredEids;
    
    // Mapping from chain name to EID for reverse lookup
    mapping(string => uint32) private _chainNameToEid;
    
    constructor() {
        _registerAllChains();
    }
    
    /// @notice Register all supported chains from the AddressBook
    function _registerAllChains() private {"""
    
    lines = [registry_header]
    
    # Group chains by category
    mainnet_chains = []
    testnet_chains = []
    sandbox_chains = []
    skipped_chains = []
    
    for chain_key in chains_processed:
        chain_data = metadata.get(chain_key, {})
        deployments = chain_data.get('deployments', [])
        
        # Find V2 deployment (prefer higher EID if multiple exist)
        v2_deployment = None
        for deployment in deployments:
            if deployment.get('version') == 2:
                if v2_deployment is None:
                    v2_deployment = deployment
                else:
                    # If we have multiple V2 deployments, prefer the one with higher EID
                    current_eid = deployment.get('eid', 0)
                    existing_eid = v2_deployment.get('eid', 0)
                    if current_eid > existing_eid:
                        v2_deployment = deployment
        
        if not v2_deployment:
            continue
            
        # Check if chain has nativeChainId
        chain_details = chain_data.get('chainDetails', {})
        native_chain_id = chain_details.get('nativeChainId', 0)
        
        if not native_chain_id:
            skipped_chains.append(chain_key)
            continue
            
        # Also check if v2_deployment has required addresses
        required_fields = ['endpointV2', 'sendUln302', 'receiveUln302', 'executor']
        missing_fields = []
        for field in required_fields:
            if field not in v2_deployment or not v2_deployment[field].get('address'):
                missing_fields.append(field)
        
        if missing_fields:
            skipped_chains.append(f"{chain_key} (missing: {', '.join(missing_fields)})")
            continue
            
        chain_name = sanitize_chain_name(chain_key)
        
        if 'sandbox' in chain_key.lower():
            sandbox_chains.append((chain_key, chain_name))
        elif 'testnet' in chain_key.lower():
            testnet_chains.append((chain_key, chain_name))
        else:
            mainnet_chains.append((chain_key, chain_name))
    
    # Add mainnet chains
    if mainnet_chains:
        lines.append("        // Mainnets")
        for chain_key, chain_name in sorted(mainnet_chains):
            lib_name = f"LayerZeroV2{chain_name}"
            lines.append(f"        _registerChain({lib_name}.EID, address({lib_name}.ENDPOINT_V2), address({lib_name}.SEND_ULN_302), address({lib_name}.RECEIVE_ULN_302), {lib_name}.EXECUTOR, {lib_name}.CHAIN_ID, \"{chain_key}\");")
    
    # Add testnet chains
    if testnet_chains:
        lines.append("\n        // Testnets")
        for chain_key, chain_name in sorted(testnet_chains):
            lib_name = f"LayerZeroV2{chain_name}"
            lines.append(f"        _registerChain({lib_name}.EID, address({lib_name}.ENDPOINT_V2), address({lib_name}.SEND_ULN_302), address({lib_name}.RECEIVE_ULN_302), {lib_name}.EXECUTOR, {lib_name}.CHAIN_ID, \"{chain_key}\");")
    
    # Add sandbox chains
    if sandbox_chains:
        lines.append("\n        // Sandbox environments")
        for chain_key, chain_name in sorted(sandbox_chains):
            lib_name = f"LayerZeroV2{chain_name}"
            lines.append(f"        _registerChain({lib_name}.EID, address({lib_name}.ENDPOINT_V2), address({lib_name}.SEND_ULN_302), address({lib_name}.RECEIVE_ULN_302), {lib_name}.EXECUTOR, {lib_name}.CHAIN_ID, \"{chain_key}\");")
    
    lines.append("""    }
    
    /// @notice Register a single chain
    function _registerChain(
        uint32 eid,
        address endpoint,
        address sendUln,
        address receiveUln,
        address executor,
        uint256 chainId,
        string memory chainName
    ) private {
        _protocolAddresses[eid] = ProtocolAddresses({
            endpointV2: endpoint,
            sendUln302: sendUln,
            receiveUln302: receiveUln,
            executor: executor,
            chainId: chainId,
            chainName: chainName,
            exists: true
        });
        _registeredEids.push(eid);
        _chainNameToEid[chainName] = eid;
    }
    
    function getProtocolAddresses(uint32 eid) public view override returns (ProtocolAddresses memory) {
        ProtocolAddresses memory addresses = _protocolAddresses[eid];
        require(addresses.exists, string.concat("Chain not registered: ", _toString(eid)));
        return addresses;
    }
    
    function isChainSupported(uint32 eid) public view override returns (bool) {
        return _protocolAddresses[eid].exists;
    }
    
    function getSupportedEids() public view override returns (uint32[] memory) {
        return _registeredEids;
    }
    
    /// @notice Get EID by chain name
    function getEidByChainName(string memory chainName) public view returns (uint32) {
        uint32 eid = _chainNameToEid[chainName];
        require(eid != 0 || _protocolAddresses[0].exists, "Chain name not found");
        return eid;
    }
    
    /// @notice Get EID by chain ID
    function getEidByChainId(uint256 chainId) public view returns (uint32) {
        // Auto-generated chain ID to EID mappings""")
    
    # Build chain ID to EID mappings from processed chains
    chain_id_to_eid = []
    for chain_key in chains_processed:
        chain_data = metadata.get(chain_key, {})
        chain_details = chain_data.get('chainDetails', {})
        native_chain_id = chain_details.get('nativeChainId', 0)
        
        # Find V2 deployment (prefer higher EID if multiple exist)
        deployments = chain_data.get('deployments', [])
        v2_deployment = None
        for deployment in deployments:
            if deployment.get('version') == 2:
                if v2_deployment is None:
                    v2_deployment = deployment
                else:
                    # If we have multiple V2 deployments, prefer the one with higher EID
                    current_eid = deployment.get('eid', 0)
                    existing_eid = v2_deployment.get('eid', 0)
                    if current_eid > existing_eid:
                        v2_deployment = deployment
        
        if v2_deployment and 'eid' in v2_deployment and native_chain_id:
            eid = v2_deployment['eid']
            chain_id_to_eid.append((native_chain_id, eid, chain_key))
    
    # Add the auto-generated chain ID mappings
    for chain_id, eid, chain_key in sorted(chain_id_to_eid):
        lines.append(f'        if (chainId == {chain_id}) return {eid}; // {chain_key}')
    
    lines.append("""        
        revert("Unsupported chain ID");
    }
    
    /// @notice Convert uint to string (helper function)
    function _toString(uint256 value) private pure returns (string memory) {
        if (value == 0) {
            return "0";
        }
        uint256 temp = value;
        uint256 digits;
        while (temp != 0) {
            digits++;
            temp /= 10;
        }
        bytes memory buffer = new bytes(digits);
        while (value != 0) {
            digits -= 1;
            buffer[digits] = bytes1(uint8(48 + uint256(value % 10)));
            value /= 10;
        }
        return string(buffer);
    }
}""")
    
    return "\n".join(lines)

def generate_lz_workers(chains_processed: List[str], metadata: Dict[str, Any]) -> str:
    """Generate the LZWorkers contract."""
    registry_header = """// SPDX-License-Identifier: MIT
pragma solidity ^0.8.22;

import "./interfaces/ILZWorkers.sol";

/// @title LayerZero Workers Registry
/// @notice Provides a registry of worker addresses (DVNs, executors) indexed by name and chain
/// @dev Maps worker names to their addresses on each chain for easy lookup
contract LZWorkers is ILZWorkers {
    // Storage: nested mapping for efficient lookups (dvnName => eid => address)
    mapping(string => mapping(uint32 => address)) private _dvnAddresses;
    
    // List of all DVN names for enumeration
    string[] private _dvnNames;
    mapping(string => bool) private _dvnNameExists;
    
    // Reverse mapping for chain name lookups
    mapping(string => uint32) private _chainNameToEid;
    mapping(uint32 => string) private _eidToChainName;
    
    // Track DVNs per chain for enumeration
    mapping(uint32 => string[]) private _dvnsByChain;
    
    constructor() {
        _registerAllDVNs();
        _registerChainMappings();
    }
    
    /// @notice Register all DVNs from metadata
    function _registerAllDVNs() private {"""
    
    lines = [registry_header]
    
    # Collect all unique DVN names across all chains
    all_dvns = {}  # canonical_name -> chain_key -> address
    
    for chain_key in chains_processed:
        chain_data = metadata.get(chain_key, {})
        dvns = chain_data.get('dvns', {})
        
        # Get V2 deployment for EID (prefer higher EID if multiple exist)
        deployments = chain_data.get('deployments', [])
        v2_deployment = None
        for deployment in deployments:
            if deployment.get('version') == 2:
                if v2_deployment is None:
                    v2_deployment = deployment
                else:
                    # If we have multiple V2 deployments, prefer the one with higher EID
                    current_eid = deployment.get('eid', 0)
                    existing_eid = v2_deployment.get('eid', 0)
                    if current_eid > existing_eid:
                        v2_deployment = deployment
        
        if not v2_deployment or 'eid' not in v2_deployment:
            continue
            
        eid = v2_deployment['eid']
        
        for address, dvn_info in dvns.items():
            canonical_name = dvn_info.get('canonicalName', '')
            deprecated = dvn_info.get('deprecated', False)
            lz_read_compatible = dvn_info.get('lzReadCompatible', False)
            
            # Skip if no canonical name, deprecated, or read-only
            if not canonical_name or deprecated or lz_read_compatible:
                continue
                
            # Skip if not a valid Ethereum address
            if not is_valid_ethereum_address(address):
                continue
                
            # Initialize DVN entry if needed
            if canonical_name not in all_dvns:
                all_dvns[canonical_name] = {}
            
            # Store the address for this chain
            all_dvns[canonical_name][chain_key] = {
                'address': to_checksum_address(address),
                'eid': eid
            }
    
    # Sort DVNs by canonical name for consistent output
    sorted_dvn_names = sorted(all_dvns.keys())
    
    # Group registrations by DVN
    for dvn_name in sorted_dvn_names:
        lines.append(f"\n        // {dvn_name}")
        chain_entries = all_dvns[dvn_name]
        
        # Sort chains for consistent output
        for chain_key in sorted(chain_entries.keys()):
            entry = chain_entries[chain_key]
            lines.append(f'        _registerDVN("{dvn_name}", {entry["eid"]}, {entry["address"]}); // {chain_key}')
    
    lines.append("""    }
    
    /// @notice Register chain name to EID mappings
    function _registerChainMappings() private {""")
    
    # Add chain mappings and collect for getEidByChainId
    chain_mappings = []
    chain_id_to_eid = []  # For getEidByChainId function
    
    for chain_key in chains_processed:
        chain_data = metadata.get(chain_key, {})
        deployments = chain_data.get('deployments', [])
        chain_details = chain_data.get('chainDetails', {})
        native_chain_id = chain_details.get('nativeChainId', 0)
        
        v2_deployment = None
        for deployment in deployments:
            if deployment.get('version') == 2:
                if v2_deployment is None:
                    v2_deployment = deployment
                else:
                    # If we have multiple V2 deployments, prefer the one with higher EID
                    current_eid = deployment.get('eid', 0)
                    existing_eid = v2_deployment.get('eid', 0)
                    if current_eid > existing_eid:
                        v2_deployment = deployment
        
        if v2_deployment and 'eid' in v2_deployment and native_chain_id:
            eid = v2_deployment['eid']
            chain_mappings.append((chain_key, eid))
            chain_id_to_eid.append((native_chain_id, eid, chain_key))
    
    # Sort by chain name for consistent output
    for chain_key, eid in sorted(chain_mappings):
        lines.append(f'        _chainNameToEid["{chain_key}"] = {eid};')
        lines.append(f'        _eidToChainName[{eid}] = "{chain_key}";')
    
    lines.append("""    }
    
    /// @notice Register a single DVN
    function _registerDVN(string memory dvnName, uint32 eid, address dvnAddress) private {
        _dvnAddresses[dvnName][eid] = dvnAddress;
        
        // Track unique DVN names
        if (!_dvnNameExists[dvnName]) {
            _dvnNameExists[dvnName] = true;
            _dvnNames.push(dvnName);
        }
        
        // Track DVNs per chain
        _dvnsByChain[eid].push(dvnName);
    }
    
    function getDVNAddress(string memory dvnName, uint32 eid) public view returns (address dvnAddress) {
        dvnAddress = _dvnAddresses[dvnName][eid];
        require(dvnAddress != address(0), string.concat("DVN not found: ", dvnName, " on chain ", _toString(eid)));
    }
    
    function getDVNAddressByChainName(string memory dvnName, string memory chainName) public view returns (address dvnAddress) {
        uint32 eid = _chainNameToEid[chainName];
        require(eid != 0, string.concat("Unknown chain: ", chainName));
        return getDVNAddress(dvnName, eid);
    }
    
    function dvnExists(string memory dvnName, uint32 eid) public view returns (bool exists) {
        return _dvnAddresses[dvnName][eid] != address(0);
    }
    
    function getAvailableDVNs() public view returns (string[] memory dvnNames) {
        return _dvnNames;
    }
    
    function getDVNsForChain(uint32 eid) public view returns (string[] memory names, address[] memory addresses) {
        names = _dvnsByChain[eid];
        addresses = new address[](names.length);
        
        for (uint256 i = 0; i < names.length; i++) {
            addresses[i] = _dvnAddresses[names[i]][eid];
        }
    }
    
    function getDVNsForChainByName(string memory chainName) public view returns (string[] memory names, address[] memory addresses) {
        uint32 eid = _chainNameToEid[chainName];
        require(eid != 0, string.concat("Unknown chain: ", chainName));
        return getDVNsForChain(eid);
    }
    
    /// @notice Convert uint to string (helper function)
    function _toString(uint256 value) private pure returns (string memory) {
        if (value == 0) {
            return "0";
        }
        uint256 temp = value;
        uint256 digits;
        while (temp != 0) {
            digits++;
            temp /= 10;
        }
        bytes memory buffer = new bytes(digits);
        while (value != 0) {
            digits -= 1;
            buffer[digits] = bytes1(uint8(48 + uint256(value % 10)));
            value /= 10;
        }
        return string(buffer);
    }
}""")
    
    return "\n".join(lines)

def generate_contracts(output_file: str = "LZAddresses.sol", 
                      protocol_file: str = "LZProtocol.sol",
                      chain_filter: Optional[List[str]] = None,
                      include_testnet: bool = False):
    """Generate Solidity contracts from LayerZero metadata."""
    # Fetch metadata
    metadata = fetch_metadata()
    
    # Generate DATA_HASH for provenance tracking
    metadata_json = json.dumps(metadata, sort_keys=True)
    data_hash = hashlib.sha256(metadata_json.encode()).hexdigest()
    timestamp = datetime.utcnow().isoformat()
    
    # Create custom header with DATA_HASH for LZAddresses.sol
    registry_header = f"""// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

// Auto-generated from LayerZero metadata - do not edit manually
// Source: {METADATA_URL}
// Generated: {timestamp}
// DATA_HASH: 0x{data_hash}

import {{ILayerZeroEndpointV2}} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/ILayerZeroEndpointV2.sol";
import {{IMessageLib}} from "@layerzerolabs/lz-evm-protocol-v2/contracts/interfaces/IMessageLib.sol";

// DATA_HASH for provenance tracking (LZIP spec requirement)
bytes32 constant LZ_ADDRESSES_DATA_HASH = 0x{data_hash};
"""
    
    # Start building the complete file
    # Use custom header if output file is for the addresses
    if "LZAddresses" in output_file:
        content = [registry_header]
    else:
        content = [SOLIDITY_HEADER]
    
    # Process each chain
    chains_processed = []
    dvn_chains_processed = []
    
    for chain_key, chain_data in sorted(metadata.items()):
        # Skip if filtering is enabled and chain not in filter
        if chain_filter and chain_key not in chain_filter:
            continue
        
        # Skip testnet chains unless specifically requested
        if 'testnet' in chain_key and not (include_testnet or (chain_filter and chain_key in chain_filter)):
            continue
        
        # Skip sandbox chains unless specifically requested
        if 'sandbox' in chain_key and not (include_testnet or (chain_filter and chain_key in chain_filter)):
            continue
        
        # Skip non-EVM chains
        chain_details = chain_data.get('chainDetails', {})
        chain_type = chain_details.get('chainType', 'evm')
        if chain_type != 'evm':
            print(f"Skipping non-EVM chain: {chain_key} (type: {chain_type})")
            continue
        
        # Generate main chain library
        chain_library = generate_chain_library(chain_key, chain_data)
        if chain_library:
            content.append(chain_library)
            chains_processed.append(chain_key)
        
        # Generate DVN library if DVNs exist
        dvns = chain_data.get('dvns', {})
        if dvns:
            dvn_library = generate_dvn_library(chain_key, dvns)
            if dvn_library:
                content.append(dvn_library)
                dvn_chains_processed.append(chain_key)
    
    # Write address book to file
    with open(output_file, 'w') as f:
        f.write('\n'.join(content))
    
    print(f"\nGenerated {output_file}")
    print(f"Processed {len(chains_processed)} chains: {', '.join(chains_processed)}")
    print(f"Generated DVN libraries for {len(dvn_chains_processed)} chains")
    
    # Generate and write LZProtocol
    protocol_content = generate_lz_protocol(chains_processed, metadata)
    with open(protocol_file, 'w') as f:
        f.write(protocol_content)
    
    print(f"\nGenerated {protocol_file}")
    
    # Count chains registered vs skipped
    registered_count = 0
    skipped_chains = []
    for chain_key in chains_processed:
        chain_data = metadata.get(chain_key, {})
        chain_details = chain_data.get('chainDetails', {})
        native_chain_id = chain_details.get('nativeChainId', 0)
        
        # Find V2 deployment (prefer higher EID if multiple exist)
        deployments = chain_data.get('deployments', [])
        v2_deployment = None
        for deployment in deployments:
            if deployment.get('version') == 2:
                if v2_deployment is None:
                    v2_deployment = deployment
                else:
                    # If we have multiple V2 deployments, prefer the one with higher EID
                    current_eid = deployment.get('eid', 0)
                    existing_eid = v2_deployment.get('eid', 0)
                    if current_eid > existing_eid:
                        v2_deployment = deployment
        
        if not native_chain_id:
            skipped_chains.append(f"{chain_key} (no chainId)")
        elif not v2_deployment:
            skipped_chains.append(f"{chain_key} (no v2 deployment)")
        else:
            # Check required fields
            required_fields = ['endpointV2', 'sendUln302', 'receiveUln302', 'executor']
            missing_fields = []
            for field in required_fields:
                if field not in v2_deployment or not v2_deployment[field].get('address'):
                    missing_fields.append(field)
            
            if missing_fields:
                skipped_chains.append(f"{chain_key} (missing: {', '.join(missing_fields)})")
            else:
                registered_count += 1
    
    print(f"Registered {registered_count} chains in LZProtocol")
    if skipped_chains:
        print(f"Skipped {len(skipped_chains)} chains:")
        for chain in skipped_chains:
            print(f"  - {chain}")
    
    # Generate and write LZWorkers
    workers_file = protocol_file.replace("LZProtocol.sol", "LZWorkers.sol")
    workers_content = generate_lz_workers(chains_processed, metadata)
    with open(workers_file, 'w') as f:
        f.write(workers_content)
    
    print(f"\nGenerated {workers_file}")
    
    # Count DVNs registered
    dvn_count = 0
    dvn_chain_count = 0
    for chain_key in chains_processed:
        chain_data = metadata.get(chain_key, {})
        dvns = chain_data.get('dvns', {})
        
        for address, dvn_info in dvns.items():
            if (not dvn_info.get('deprecated', False) and 
                not dvn_info.get('lzReadCompatible', False) and
                dvn_info.get('canonicalName')):
                dvn_count += 1
                dvn_chain_count += 1
    
    print(f"Registered DVNs across {len(chains_processed)} chains")

def main():
    """Main entry point."""
    import argparse
    
    parser = argparse.ArgumentParser(description='Generate LayerZero V2 contracts from metadata')
    parser.add_argument('-o', '--output', default='src/src-gen/LZAddresses.sol',
                        help='Output file name for address book (default: LZAddresses.sol)')
    parser.add_argument('-p', '--protocol', default='LZProtocol.sol',
                        help='Output file name for protocol addresses (default: LZProtocol.sol)')
    parser.add_argument('-w', '--workers', default='LZWorkers.sol',
                        help='Output file name for worker addresses (default: LZWorkers.sol)')
    parser.add_argument('-c', '--chains', nargs='+',
                        help='Filter to specific chains (e.g., base-mainnet arbitrum-mainnet)')
    parser.add_argument('--include-testnet', action='store_true',
                        help='Include testnet and sandbox chains')
    
    args = parser.parse_args()
    
    generate_contracts(
        output_file=args.output,
        protocol_file=args.protocol,
        chain_filter=args.chains,
        include_testnet=args.include_testnet
    )

if __name__ == "__main__":
    main()