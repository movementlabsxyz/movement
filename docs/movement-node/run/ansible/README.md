

1. Get the container version
```bash
# From /home/${USER}
git clone https://github.com/movementlabsxyz/movement.git
cd movement
CONTAINER_REV=$(git rev-parse HEAD)
echo "CONTAINER_REV=${CONTAINER_REV}"
```


2. Run the playbook
Make sure you can connect to the host where you want test or like in this case, a test
ec2 instance
```bash
ssh ubuntu@ec2-54-215-191-59.us-west-1.compute.amazonaws.com
```

```bash
ansible-playbook --inventory ec2-54-215-191-59.us-west-1.compute.amazonaws.com, \
                 --user ubuntu  \
                 --extra-vars "movement_container_version="${CONTAINER_REV}"" \
                 docs/movement-node/run/ansible/suzuka-full-node.yml
```
