use maptos_framework_release_util::commit_hash_with_script;

commit_hash_with_script!(
	PostL1Merge,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"21ec1f2d4367952c103706f51652d47379ce9b1a",          // Commit hash
	6,                                                   // Bytecode version
	"post-l1-merge.mrb",                                 // MRB file name
	"POST_L1_MERGE_FRAMEWORK_RELEASE"                    // Cache environment variable for Elsa
);
