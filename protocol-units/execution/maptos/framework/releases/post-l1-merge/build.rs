use maptos_framework_release_util::commit_hash_with_script;

commit_hash_with_script!(
	PostL1Merge,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"d00f5e5ef3179919b3fc8245ac774f8509ed6a3e",          // Commit hash
	6,                                                   // Bytecode version
	"biarritz-rc1.mrb",                                  // MRB file name
	"CACHE_POST_L1_MERGE_FRAMEWORK_RELEASE"              // Cache environment variable for Elsa
);
