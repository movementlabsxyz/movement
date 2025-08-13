use maptos_framework_release_util::commit_hash_with_script;

// Example usage of the macro to generate a build script for Elsa.
// change
commit_hash_with_script!(
	BiarritzRc1,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"c5d8d936b7775436ff6c256e10049b4de497c220",          // Commit hash
	6,                                                   // Bytecode version
	"pre-l1-merge.mrb",                                  // MRB file name
	"CACHE_PRE_L1_MERGE_FRAMEWORK_RELEASE"               // Cache environment variable
);
