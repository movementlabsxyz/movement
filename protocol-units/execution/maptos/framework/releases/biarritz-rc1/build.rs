use maptos_framework_release_util::commit_hash_with_script;

// Example usage of the macro to generate a build script for Elsa.
// change
commit_hash_with_script!(
	BiarritzRc1,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"27397b5835e6a466c06c884a395653c9ff13d1fe",          // Commit hash
	6,                                                   // Bytecode version
	"biarritz-rc1.mrb",                                  // MRB file name
	"CACHE_BIARRITZ_RC1_FRAMEWORK_RELEASE"               // Cache environment variable for Elsa
);
