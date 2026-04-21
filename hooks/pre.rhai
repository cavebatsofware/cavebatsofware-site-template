// cargo-generate pre-generation hook.
// Most substitution is handled by liquid templating inline.
// This hook echoes the chosen options for visibility during generation.

let project_name = variable::get("project-name");
let license_style = variable::get("license-style");

print(`[pre-hook] generating project: ${project_name}`);
print(`[pre-hook] license-style: ${license_style}`);
