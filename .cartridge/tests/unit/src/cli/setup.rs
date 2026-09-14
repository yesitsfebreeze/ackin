use super::*;

/// `$CARTRIDGE_HOME` is process-global: the tests that point it somewhere
/// take turns, so one test's store is never another's mid-run.
pub(crate) fn trust_home() -> std::sync::MutexGuard<'static, ()> {
	static TURNS: std::sync::Mutex<()> = std::sync::Mutex::new(());
	TURNS.lock().unwrap()
}

/// A cartridge folder `under/<name>.ctg`: `more` is laid over the manifest,
/// and `lua` is its `init.lua`.
fn cartridge(under: &Path, name: &str, description: &str, more: Value, lua: &str) {
	let dir = under.join(format!("{name}.ctg"));
	std::fs::create_dir_all(&dir).unwrap();
	let mut manifest = json!({"name": name, "description": description, "entry": "init.lua"});
	cartridge::settings::merge(&mut manifest, more);
	std::fs::write(dir.join(MANIFEST), manifest.to_string()).unwrap();
	std::fs::write(dir.join("init.lua"), lua).unwrap();
}

/// The test binary is not the base; nodes run the built `cartridge`.
fn node_binary() {
	let output = std::process::Command::new(env!("CARGO"))
		.args(["build", "--bin", "cartridge", "--message-format=json"])
		.current_dir(env!("CARGO_MANIFEST_DIR"))
		.output()
		.unwrap();
	assert!(
		output.status.success(),
		"{}",
		String::from_utf8_lossy(&output.stderr)
	);
	let bin = String::from_utf8(output.stdout)
		.unwrap()
		.lines()
		.filter_map(|line| serde_json::from_str::<Value>(line).ok())
		.find_map(|m| m["executable"].as_str().map(str::to_owned))
		.expect("cargo build produced the cartridge binary");
	std::env::set_var(cartridge::host::NODE_BIN_ENV, bin);
}

#[test]
fn setup_offers_what_it_finds_and_the_catalog_and_filters_by_subsequence() {
	let tmp = tempfile::tempdir().unwrap();
	let checkouts = tmp.path().join("checkouts");
	cartridge(&checkouts, "alpha", "First.", json!({}), "");
	cartridge(&checkouts, "beta", "Second.", json!({}), "");
	// A nested cartridge belongs to its parent and is not offered on its own.
	cartridge(
		&checkouts.join("beta.ctg"),
		"inner",
		"Nested.",
		json!({}),
		"",
	);
	let catalog = tmp.path().join("catalog.json");
	std::fs::write(
		&catalog,
		r#"{"beta":{"repository":"https://example.test/beta.git","description":"Known."},
		    "gamma":{"repository":"https://example.test/gamma.git","description":"Third, a model thing."}}"#,
	)
	.unwrap();

	let known = read_catalog(&catalog).unwrap();
	assert_eq!(known.len(), 2);
	let found = candidates(std::slice::from_ref(&checkouts), known);
	assert_eq!(
		found.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
		["alpha", "beta", "gamma"]
	);
	// On disk wins over the catalog for the same name.
	assert_eq!(found[1].description, "Second.");
	assert!(matches!(found[1].source, Source::Folder(_)));
	assert_eq!(
		found[2].source,
		Source::Repository("https://example.test/gamma.git".into())
	);

	let all: Vec<usize> = (0..found.len()).collect();
	assert_eq!(select(&found, &all, "2").unwrap()[0].name, "beta");
	assert_eq!(select(&found, &all, "gamma, 1").unwrap().len(), 2);
	assert!(select(&found, &all, "4").is_err());
	assert!(select(&found, &all, "delta").is_err());
	assert_eq!(select(&found, &all, "all").unwrap().len(), 3);
	assert_eq!(select(&found, &[2], "").unwrap()[0].name, "gamma");
	assert!(select(&found, &all, "none").unwrap().is_empty());
	assert!(matches(&found[2], "mdl"));
	assert!(matches(&found[0], "ALPHA"));
	assert!(!matches(&found[0], "gamma"));
	assert!(read_catalog(&tmp.path().join("missing.json"))
		.unwrap()
		.is_empty());
}

#[test]
fn setup_links_the_chosen_writes_a_profile_the_host_reads_and_lets_a_cartridge_ask() {
	let _turn = trust_home();
	let tmp = tempfile::tempdir().unwrap();
	std::env::set_var("CARTRIDGE_HOME", tmp.path().join("home"));
	let checkouts = tmp.path().join("checkouts");
	cartridge(&checkouts, "alpha", "First.", json!({}), "");
	// A cartridge with a setup exchange: one question with a default, then
	// the configuration to write, and a doctor that reports on it.
	cartridge(
		&checkouts,
		"beta",
		"Second.",
		json!({
			"events": {"beta.setup": {}, "beta.doctor": {}},
			"listen": ["beta.setup", "beta.doctor"],
			"setup": "beta.setup",
			"doctor": "beta.doctor",
		}),
		r#"
		cartridge.listen("beta.setup", function(input)
			if input.answers.port == nil then
				return { ask = { { key = "port", prompt = "Which port?", default = 4242 } } }
			end
			return { done = true, config = { port = input.answers.port }, note = "port chosen" }
		end)
		cartridge.listen("beta.doctor", function() return { ok = false, problems = { "no key" } } end)
		"#,
	);

	let project = tmp.path().join("project");
	std::fs::create_dir_all(&project).unwrap();
	let builtin = project.join("builtin");
	let found = candidates(std::slice::from_ref(&checkouts), Vec::new());
	let chosen = select(&found, &[0, 1], "beta").unwrap();

	let installed = install(&builtin, &chosen).unwrap();
	let link = std::fs::read_link(builtin.join("beta")).unwrap();
	assert!(link.is_relative(), "{}", link.display());
	assert_eq!(
		std::fs::canonicalize(builtin.join("beta")).unwrap(),
		std::fs::canonicalize(checkouts.join("beta.ctg")).unwrap()
	);
	// Installing again is idempotent.
	install(&builtin, &chosen).unwrap();
	assert_eq!(std::fs::read_link(builtin.join("beta")).unwrap(), link);

	let done = write(&project, &builtin, &installed).unwrap();
	assert_eq!(done.len(), 3, "{done:?}");
	let init = std::fs::read_to_string(project.join(".cartridge/init.lua")).unwrap();
	assert!(init.contains(r#"{ id = "beta", path = "beta" }"#), "{init}");
	assert!(project.join(".cartridge/.gitignore").is_file());

	// The profile it wrote is one the host composes.
	let host = Host::new(&builtin, project.join(".cartridge")).unwrap();
	let enabled = host.entries().unwrap();
	assert_eq!(
		enabled
			.iter()
			.filter(|e| !e.disabled)
			.map(|e| e.id.as_str())
			.collect::<Vec<_>>(),
		["beta"]
	);
	drop(host);

	// Without a terminal the question takes its default and the answer lands
	// in config.lua under the entry id.
	node_binary();
	let rt = tokio::runtime::Runtime::new().unwrap();
	let report = rt
		.block_on(run_setups(&project, &builtin, &installed, false))
		.unwrap();
	assert!(
		report.iter().any(|l| l == "beta: port chosen"),
		"{report:?}"
	);
	let config = cartridge::settings::read(&project.join(".cartridge/config.lua")).unwrap();
	assert_eq!(config["beta"]["port"], 4242, "{config}");

	// The doctor asks the same cartridge and reports its verdict.
	let project = Project {
		dir: builtin.clone(),
		profile: project.join(".cartridge"),
	};
	let exit = rt.block_on(doctor(&project)).unwrap();
	assert_eq!(exit, ExitCode::from(FAILED));
}

#[test]
fn a_folder_in_the_way_of_a_link_is_refused() {
	let tmp = tempfile::tempdir().unwrap();
	let checkouts = tmp.path().join("checkouts");
	cartridge(&checkouts, "alpha", "First.", json!({}), "");
	let builtin = tmp.path().join("project/builtin");
	std::fs::create_dir_all(builtin.join("alpha")).unwrap();
	let found = candidates(&[checkouts], Vec::new());
	let error = install(&builtin, &found).unwrap_err().to_string();
	assert!(error.contains("is not a link"), "{error}");
}

#[test]
fn a_setup_or_doctor_event_the_cartridge_does_not_listen_to_is_refused() {
	let tmp = tempfile::tempdir().unwrap();
	for field in ["setup", "doctor"] {
		cartridge(tmp.path(), field, "", json!({ field: "x" }), "");
		let manifest = tmp.path().join(format!("{field}.ctg")).join(MANIFEST);
		let error = Cartridge::document(&manifest).err().unwrap().to_string();
		assert!(error.contains("does not listen to"), "{error}");
	}
}

/// The record after the exchange approves nothing a running cartridge may
/// have written: a file that changed while the cartridges ran fails setup.
#[test]
fn a_file_changed_by_the_exchange_is_not_recorded() {
	let _turn = trust_home();
	let tmp = tempfile::tempdir().unwrap();
	std::env::set_var("CARTRIDGE_HOME", tmp.path().join("home"));
	let root = tmp.path().join("project");
	cartridge(&root, "one", "", json!({}), "");
	std::fs::create_dir_all(root.join(".cartridge")).unwrap();
	std::fs::write(root.join(".cartridge/init.lua"), "return {}").unwrap();
	cartridge::trust::record(&root).unwrap();
	// What a cartridge with a write grant could have written while it ran.
	cartridge(
		&root,
		"sneaky",
		"",
		json!({ "grant": { "exec": ["*"] } }),
		"",
	);
	let error = strayed(&root, &root.join(".cartridge/config.lua"))
		.unwrap_err()
		.to_string();
	assert!(
		error.contains("changed while the setup exchange ran"),
		"{error}"
	);
}

/// Choosing is the approval: setup records the profile it wrote and each
/// cartridge it chose — a folder the tree already held stays untrusted, and
/// so does a config.lua setup did not write.
#[test]
fn setup_trusts_what_it_chose_not_what_the_tree_holds() {
	let _turn = trust_home();
	let tmp = tempfile::tempdir().unwrap();
	std::env::set_var("CARTRIDGE_HOME", tmp.path().join("home"));
	let checkouts = tmp.path().join("checkouts");
	cartridge(&checkouts, "alpha", "First.", json!({}), "");
	let project = tmp.path().join("project");
	std::fs::create_dir_all(&project).unwrap();
	let builtin = project.join("builtin");
	// What a clone could bring along: a folder nobody chose, and a config.
	cartridge(
		&builtin,
		"stray",
		"",
		json!({ "grant": { "exec": ["*"] } }),
		"",
	);
	std::fs::create_dir_all(project.join(".cartridge")).unwrap();
	std::fs::write(project.join(".cartridge/config.lua"), "return {}").unwrap();

	let found = candidates(std::slice::from_ref(&checkouts), Vec::new());
	let installed = install(&builtin, &found).unwrap();
	write(&project, &builtin, &installed).unwrap();

	cartridge::trust::verify(&builtin.join("alpha").join(MANIFEST)).unwrap();
	cartridge::trust::verify(&project.join(".cartridge/init.lua")).unwrap();
	let refused = cartridge::trust::verify(&builtin.join("stray.ctg").join(MANIFEST))
		.unwrap_err()
		.to_string();
	assert!(refused.contains("run `cartridge trust"), "{refused}");
	let refused = cartridge::trust::verify(&project.join(".cartridge/config.lua"))
		.unwrap_err()
		.to_string();
	assert!(refused.contains("run `cartridge trust"), "{refused}");
}
