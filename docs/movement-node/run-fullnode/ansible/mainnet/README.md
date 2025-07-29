# Mainnet Full Nodes
We've provided a hardcoded Ansible playbook for joining the Movement Mainnet as a full node. 

```shell
ansible-playbook --inventory <your-inventory> \
    --user ubuntu  \
    --extra-vars "movement_container_version=${CONTAINER_REV}" \
    --extra-vars "movement_repo_commit=${MOVEMENT_COMMIT}" \
    --extra-vars "user=ubuntu" \
    docs/movement-node/run-fullnode/ansible/mainnet/movement-fullnode.yml \
    --private-key your-private-key.pem
```