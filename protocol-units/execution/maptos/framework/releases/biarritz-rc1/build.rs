use maptos_framework_release_util::commit_hash_with_script;

// Example usage of the macro to generate a build script for Elsa.
// change
commit_hash_with_script!(
	BiarritzRc1,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"4e88300b5d3356bcdd4ae98d3ea6335a3ad03321",          // Commit hash
	6,                                                   // Bytecode version
	"biarritz-rc1.mrb",                                  // MRB file name
	"CACHE_BIARRITZ_RC1_FRAMEWORK_RELEASE"               // Cache environment variable for Elsa
);
