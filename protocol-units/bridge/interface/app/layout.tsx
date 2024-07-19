import type { Metadata } from "next";
import "./globals.css";
import { headers } from 'next/headers'
import { cookieToInitialState } from 'wagmi'
import { config } from '@/config'
import Web3ModalProvider from '@/context'

export const metadata: Metadata = {
  title: "Movement Bridge App",
  description: "Bridge Developed by Movement Labs serving as a gateway to the Movement Ecosystem",
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  const initialState = cookieToInitialState(config, headers().get('cookie'))
  return (
    <html lang="en">
      <body className="">
      <Web3ModalProvider initialState={initialState}>    
        {children}
        </Web3ModalProvider>
        </body>
        
    </html>
  );
}
