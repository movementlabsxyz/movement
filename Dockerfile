FROM ghcr.io/movementlabsxyz/celestia-app:8c881eb41e59292abbe1a4c83d69b46ac8b0e9da

ENTRYPOINT [ "/app/celestia-appd", "start", "--address", "tcp://0.0.0.0:26658", "--proxy_app", "tcp://0.0.0.0:26658", "--home", "/mnt/.movement/celestia/c64b84be0bbe1b30f0a5/.celestia-app" ]