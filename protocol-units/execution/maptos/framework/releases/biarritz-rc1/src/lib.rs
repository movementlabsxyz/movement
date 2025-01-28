use maptos_framework_release_util::commit_hash_with_script;

// Example usage of the macro to generate a build script for Elsa.
commit_hash_with_script!(
	BiarritzRc1,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"fe6432ee54f91ac09cecc59ee92e8a19baedaa57",          // Commit hash
	6,                                                   // Bytecode version
	"biarritz-rc1.mrb",                                  // MRB file name
	"CACHE_BIARRITZ_RC1_FRAMEWORK_RELEASE"               // Cache environment variable for Elsa
);
