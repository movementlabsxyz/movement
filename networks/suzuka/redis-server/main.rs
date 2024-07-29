// seems like we could only run an instance of redis-server in a separate process, and then connect to it from the other services.
// we might want to spin up a redis server then. 
// https://discourse.nixos.org/t/how-can-i-spawn-a-redis-instance-within-a-nix-build/5155
// For testing purposes we could use:
// https://medium.com/@suyashkant.srivastava/this-post-aims-to-help-setting-up-a-minimal-redis-cluster-on-nix-environment-d607e3628e08