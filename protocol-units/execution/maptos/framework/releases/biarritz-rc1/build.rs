use maptos_framework_release_util::commit_hash_with_script;

// Example usage of the macro to generate a build script for Elsa.
commit_hash_with_script!(
	BiarritzRc1,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"9dfc8e7a3d622597dfd81cc4ba480a5377f87a41",          // Commit hash
	6,                                                   // Bytecode version
	"biarritz-rc1.mrb",                                  // MRB file name
	"CACHE_ELSA_FRAMEWORK_RELEASE"                       // Cache environment variable for Elsa
);
