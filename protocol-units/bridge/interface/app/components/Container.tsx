'use client'
import React from 'react';
import '../globals.css';

const Container = ({props}:any) => {
    const [target, setTarget] = React.useState("Movement");
    const [amount, setAmount] = React.useState(0);

    return (
        <div className="bg-black text-white p-6 w-[36rem] mx-auto border-[1px] border-[#FFCF1]">
          <div className="flex justify-between items-center mb-4">
            <button className="text-gray-400 hover:text-gray-200">
              See transaction history
            </button>
          </div>
          <div className="bg-[#FFFCF1] bg-opacity-10 p-4">
            <div className="mb-4">
              
              <div className="relative">
                <div className="flex justify-between">
                <div className="flex justify-left">

                <h1 className="text-gray-400 p-2 mb-2 w-20">From:</h1>
                <select className="appearance-none bg-[#FFFCF1] bg-opacity-10 w-28 text-white p-2 mb-2">
                    <option>Ethereum</option>
                    <option>Movement</option>
                </select>
                </div>
                <button>Balance: {0} ETH</button>
                </div>
                <input
                  type="number"
                  placeholder="Enter amount"
                  className="bg-[#FFFCF1] bg-opacity-10 text-white w-full p-2 mb-2"
                />
                <div className="text-gray-400 text-sm">
                  Ethereum gas fee: 0.00094 ETH ($3.24)
                </div>
              </div>
            </div>
            <div className="flex justify-center mb-4 ">
              <button className="bg-[#FFFCF1] z-20 bg-opacity-10 p-2 hover:cursor-pointer">
                <svg
                  className="w-6 h-6 text-gray-200 rotate"
                  fill="none"
                  stroke="currentColor"
                  viewBox="0 0 24 24"
                  xmlns="http://www.w3.org/2000/svg"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth="2"
                    d="M19 9l-7 7-7-7"
                  ></path>
                </svg>
              </button>
            </div>
            <div className="mb-4">

              <div className="relative">
                <div className="flex justify-left">
              <label className="text-gray-400 p-2 mb-2 w-20">To:</label>
                <select className="appearance-none bg-[#FFFCF1] bg-opacity-10 text-white w-28 p-2 mb-2">
                    <option>Movement</option>
                    <option>Ethereum</option>
                </select>
                </div>
                <input
                  type="text"
                  placeholder="Receiving Address"
                  className="bg-[#FFFCF1] bg-opacity-10 text-white w-full p-2 mb-2"
                />

                <div className="text-gray-400 text-sm">Movement gas fee: 0 ETH ($0)</div>
              </div>
            </div>
            <div className="mb-4">
              <div className="text-gray-400 mb-2">Summary</div>
              <div className="flex justify-between text-sm">
                <div>You will pay in gas fees:</div>
                <div>0.00094 ETH ($3.24)</div>
              </div>
              <div className="flex justify-between text-sm">
                <div>You will receive on Movement:</div>
                <div>0 ETH ($0)</div>
              </div>
            </div>
            picks a wallet type depending on source chain:
            <w3m-button />
            once connected:
            <button className="bg-[#FFFCF1] bg-opacity-10 text-gray-400 w-full py-2 cursor-not-allowed" disabled>
              Move funds to Movement or Ethereum
            </button>
          </div>
        </div>
      );
    
    
};

export default Container;