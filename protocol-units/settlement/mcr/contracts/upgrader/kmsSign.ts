import { KMS } from 'aws-sdk';
import { keccak256 } from 'js-sha3';
import * as ethutil from 'ethereumjs-util';
import * as asn1 from 'asn1.js';
import BN from 'bn.js';
import { Transaction, TxData } from 'ethereumjs-tx';

const kms = new KMS({
    accessKeyId: '<access_key_id>', // credentials for your IAM user with KMS access
    secretAccessKey: '<access_secret>', // credentials for your IAM user with KMS access
    region: 'us-east-1',
    apiVersion: '2014-11-01',
});

const keyId = '<KMS key id>';

const EcdsaSigAsnParse = asn1.define('EcdsaSig', function(this: any) {
    // parsing this according to https://tools.ietf.org/html/rfc3279#section-2.2.3 
    this.seq().obj( 
        this.key('r').int(), 
        this.key('s').int(),
    );
});

const EcdsaPubKey = asn1.define('EcdsaPubKey', function(this: any) {
    // parsing this according to https://tools.ietf.org/html/rfc5480#section-2
    this.seq().obj( 
        this.key('algo').seq().obj(
            this.key('a').objid(),
            this.key('b').objid(),
        ),
        this.key('pubKey').bitstr()
    );
});

async function sign(msgHash, keyId) {
    const params : KMS.SignRequest = {
        // key id or 'Alias/<alias>'
        KeyId: keyId, 
        Message: msgHash, 
        // 'ECDSA_SHA_256' is the one compatible with ECC_SECG_P256K1.
        SigningAlgorithm: 'ECDSA_SHA_256',
        MessageType: 'DIGEST' 
    };
    const res = await kms.sign(params).promise();
    return res;
}

async function getPublicKey(keyPairId: string) {
    return kms.getPublicKey({
        KeyId: keyPairId
    }).promise();
}

function getEthereumAddress(publicKey: Buffer): string {
    console.log("Encoded Pub Key: " + publicKey.toString('hex'));

    // The public key is ASN1 encoded in a format according to 
    // https://tools.ietf.org/html/rfc5480#section-2
    // I used https://lapo.it/asn1js to figure out how to parse this 
    // and defined the schema in the EcdsaPubKey object
    let res = EcdsaPubKey.decode(publicKey, 'der');
    let pubKeyBuffer : Buffer = res.pubKey.data;

    // The public key starts with a 0x04 prefix that needs to be removed
    // more info: https://www.oreilly.com/library/view/mastering-ethereum/9781491971932/ch04.html
    pubKeyBuffer = pubKeyBuffer.slice(1, pubKeyBuffer.length);

    const address = keccak256(pubKeyBuffer) // keccak256 hash of publicKey  
    const buf2 = Buffer.from(address, 'hex');
    const EthAddr = "0x" + buf2.slice(-20).toString('hex'); // take last 20 bytes as ethereum adress
    console.log("Generated Ethreum address: " + EthAddr);
    return EthAddr;
}

async function findEthereumSig(plaintext) {
    let signature = await sign(plaintext, keyId);
    if (signature.Signature == undefined) {
        throw new Error('Signature is undefined.');
    }
    console.log("encoded sig: " + signature.Signature.toString('hex'));

    let decoded = EcdsaSigAsnParse.decode(signature.Signature, 'der');
    let r : BN = decoded.r;
    let s : BN = decoded.s;
    console.log("r: " + r.toString(10));
    console.log("s: " + s.toString(10));

    let tempsig = r.toString(16) + s.toString(16);

    let secp256k1N = new BN("fffffffffffffffffffffffffffffffebaaedce6af48a03bbfd25e8cd0364141", 16); // max value on the curve
    let secp256k1halfN = secp256k1N.div(new BN(2)); // half of the curve
    // Because of EIP-2 not all elliptic curve signatures are accepted
    // the value of s needs to be SMALLER than half of the curve
    // i.e. we need to flip s if it's greater than half of the curve
    if (s.gt(secp256k1halfN)) {
        console.log("s is on the wrong side of the curve... flipping - tempsig: " + tempsig + " length: " + tempsig.length);
        // According to EIP2 https://github.com/ethereum/EIPs/blob/master/EIPS/eip-2.md
        // if s < half the curve we need to invert it 
        // s = curve.n - s
        s = secp256k1N.sub(s);
        console.log("new s: " + s.toString(10));
        return { r, s }
    }
    // if s is less than half of the curve, we're on the "good" side of the curve, we can just return
    return { r, s }
}

function recoverPubKeyFromSig(msg: Buffer, r : BN, s : BN, v: number) {
    console.log("Recovering public key with msg " + msg.toString('hex') + " r: " + r.toString(16) + " s: " + s.toString(16));
    let rBuffer = r.toBuffer();
    let sBuffer = s.toBuffer();
    let pubKey = ethutil.ecrecover(msg, v, rBuffer, sBuffer);
    let addrBuf = ethutil.pubToAddress(pubKey);
    var RecoveredEthAddr = ethutil.bufferToHex(addrBuf);
    console.log( "Recovered ethereum address: " +  RecoveredEthAddr);
    return RecoveredEthAddr;
}

function findRightKey(msg: Buffer, r : BN, s: BN, expectedEthAddr: string) {
    // This is the wrapper function to find the right v value
    // There are two matching signatues on the elliptic curve
    // we need to find the one that matches to our public key
    // it can be v = 27 or v = 28
    let v = 27;
    let pubKey = recoverPubKeyFromSig(msg, r, s, v);
    if (pubKey != expectedEthAddr) {
        // if the pub key for v = 27 does not match
        // it has to be v = 28
        v = 28;
        pubKey = recoverPubKeyFromSig(msg, r, s, v)
    }
    console.log("Found the right ETH Address: " + pubKey + " v: " + v);
    return { pubKey, v };
}

// txTest();
// async function txTest() {
//     const web3 = new Web3(new Web3.providers.HttpProvider("https://kovan.infura.io/v3/<infura_key>"));

//     let pubKey = await getPublicKey(keyId);
//     let ethAddr = getEthereumAddress((pubKey.PublicKey as Buffer));
//     let ethAddrHash = ethutil.keccak(Buffer.from(ethAddr));
//     let sig = await findEthereumSig(ethAddrHash);
//     let recoveredPubAddr = findRightKey(ethAddrHash, sig.r, sig.s, ethAddr);

//     const txParams: TxData = {
//         nonce: await web3.eth.getTransactionCount(ethAddr),
//         gasPrice: '0x0918400000',
//         gasLimit: 160000,
//         to: '0x0000000000000000000000000000000000000000',
//         value: '0x00',
//         data: '0x00',
//         r: sig.r.toBuffer(),
//         s: sig.s.toBuffer(),
//         v: recoveredPubAddr.v
//     }

//     console.log(txParams);

//     const tx = new Transaction(txParams, {
//         chain: 'kovan',
//     });

//     let txHash = tx.hash(false);
//     sig = await findEthereumSig(txHash);
//     recoveredPubAddr = findRightKey(txHash, sig.r, sig.s, ethAddr);
//     tx.r = sig.r.toBuffer();
//     tx.s = sig.s.toBuffer();
//     tx.v = new BN(recoveredPubAddr.v).toBuffer();
//     console.log(tx.getSenderAddress().toString('hex'));

//     // Send signed tx to ethereum network 
//     const serializedTx = tx.serialize().toString('hex');    
//     web3.eth.sendSignedTransaction('0x' + serializedTx)
//     .on('confirmation', (confirmationNumber : number, receipt : TransactionReceipt) => {})
//     .on('receipt', (txReceipt : TransactionReceipt) => { 
//         console.log("signAndSendTx txReceipt. Tx Address: " + txReceipt.transactionHash);
//     })  
//     .on('error', error => console.log(error));
// }