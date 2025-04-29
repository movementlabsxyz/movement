use maptos_framework_release_util::commit_hash_with_script;

commit_hash_with_script!(
	PostL1Merge,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"035700578e23aff9bff4aba0a415cf26cf7731a5",          // Commit hash
	6,                                                   // Bytecode version
	"post-l1-merge.mrb",                                 // MRB file name
	"POST_L1_MERGE_FRAMEWORK_RELEASE"                    // Cache environment variable for Elsa
);
