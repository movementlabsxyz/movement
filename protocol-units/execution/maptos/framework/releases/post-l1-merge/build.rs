use maptos_framework_release_util::commit_hash_with_script;

commit_hash_with_script!(
	PostL1Merge,                                         // Struct name
	"https://github.com/movementlabsxyz/aptos-core.git", // Repository URL
	"f3a2758f6e13e4ac3d7e7425c576817358f9b596",          // Commit hash
	6,                                                   // Bytecode version
	"post-l1-merge.mrb",                                 // MRB file name
	"CACHE_POST_L1_MERGE_FRAMEWORK_RELEASE"              // Cache environment variable for Elsa
);
