forge script SetOFTConfigs --rpc-url https://rpc.hypurrscan.io --account movement_deployer 2>&1 | tee -a ./defaultconfig.log
forge script SetOFTConfigs --rpc-url "$(chains base)" --account movement_deployer 2>&1 | tee -a ./defaultconfig.log
