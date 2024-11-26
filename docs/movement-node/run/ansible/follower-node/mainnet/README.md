# Mainnet Follower Nodes
We've provided a hardcoded Ansible playbook for joining the Movement Mainnet as a follower node. 

```shell
ansible-playbook --inventory <your-inventory> \
    --user ubuntu  \
    --extra-vars "movement_container_version=${CONTAINER_REV}" \
    --extra-vars "user=ubuntu" \
    docs/movement-node/run/ansible/follower-node/mainnet/movement-full-follower.yml \
    --private-key your-private-key.pem
```