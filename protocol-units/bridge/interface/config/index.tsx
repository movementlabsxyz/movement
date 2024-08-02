import { defaultWagmiConfig } from '@web3modal/wagmi/react/config'

import { cookieStorage, createStorage } from 'wagmi'
import { mainnet, holesky } from 'wagmi/chains'

// Your WalletConnect Cloud project ID
export const projectId = '8a62190b423c07859fdf48224ce8285e'

// Create a metadata object
const metadata = {
  name: 'Movement Bridge',
  description: 'AppKit',
  url: 'https://bridge.movementlabs.xyz', // origin must match your domain & subdomain
  icons: ['https://avatars.githubusercontent.com/u/37784886']
}



// Create wagmiConfig
const chains = [mainnet, holesky] as const
export const config = defaultWagmiConfig({
  chains,
  projectId,
  metadata,
  ssr: true,
  storage: createStorage({
    storage: cookieStorage
  }),
//   ...wagmiOptions // Optional - Override createConfig parameters
})