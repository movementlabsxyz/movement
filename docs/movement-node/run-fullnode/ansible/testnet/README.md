# Testnet Full Nodes
We've provided a hardcoded Ansible playbook for joining the Movement Testnet as a full node. 

```shell
ansible-playbook --inventory <your-inventory> \
    --user ubuntu  \
    --extra-vars "movement_container_version=${CONTAINER_REV}" \
    --extra-vars "user=ubuntu" \
    docs/movement-node/run-fullnode/ansible/testnet/movement-node.yml \
    --private-key your-private-key.pem
```