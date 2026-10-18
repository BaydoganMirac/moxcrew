import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const directory = "crates/patchfleet-desktop/assets/windows/";
const [config, nsis, wix] = await Promise.all([
  readFile("Packager.toml", "utf8"), readFile(`${directory}installer.nsi`, "utf8"), readFile(`${directory}main.wxs`, "utf8"),
]);

function section(name) {
  return config.match(new RegExp(`\\[${name}\\]([\\s\\S]*?)(?=\\n\\[|$)`))?.[1] ?? "";
}

function nsisDefines(product, manufacturer) {
  const result = Object.fromEntries([...nsis.matchAll(/^!define (\w+) "([^"]*)"$/gm)].map((match) => [match[1], match[2]]));
  result.PRODUCTNAME = product;
  result.MANUFACTURER = manufacturer;
  const expand = (value) => value.replace(/\$\{(\w+)\}/g, (_, key) => expand(result[key]));
  return Object.fromEntries(["UNINSTKEY", "MANUPRODUCTKEY", "INSTALLIDENTITY"].map((key) => [key, expand(result[key])]));
}

test("display rename retains existing NSIS upgrade and install identities", () => {
  assert.match(section("nsis"), /template = "crates\/patchfleet-desktop\/assets\/windows\/installer.nsi"/);
  assert.match(section("nsis"), /installMode = "currentUser"/);
  for (const display of ["Patchfleet", "Moxcrew"]) {
    assert.deepEqual(nsisDefines(display, display), {
      UNINSTKEY: "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Patchfleet",
      MANUPRODUCTKEY: "Software\\Patchfleet\\Patchfleet",
      INSTALLIDENTITY: "Patchfleet",
    });
  }
  assert.match(nsis, /StrCpy \$INSTDIR "\$LOCALAPPDATA\\\$\{INSTALLIDENTITY\}"/);
  assert.doesNotMatch(nsis, /\$(?:LOCALAPPDATA|PROGRAMFILES64|PROGRAMFILES)\\\$\{PRODUCTNAME\}/);
  assert.match(nsis, /MUI_STARTMENUPAGE_DEFAULTFOLDER "\$\{INSTALLIDENTITY\}"/);
  assert.match(nsis, /"DisplayName" "\$\{PRODUCTNAME\}"/);
  assert.match(nsis, /"Publisher" "\$\{MANUFACTURER\}"/);
  assert.match(nsis, /ApplicationID::Set .*"\$\{IDENTIFIER\}"/);
});

test("NSIS detects both legacy and current WiX identity pairs without changing version flow", () => {
  assert.match(nsis, /\$R0 == "\$\{PRODUCTNAME\}"[\s\S]*\$R1 == "\$\{MANUFACTURER\}"[\s\S]*Goto wix_match/);
  assert.match(nsis, /\$R0 != "\$\{INSTALLIDENTITY\}"[\s\S]*\$R1 != "\$\{INSTALLMANUFACTURER\}"[\s\S]*Goto wix_loop/);
  assert.match(nsis, /wix_match:[\s\S]*StrCpy \$R7 "wix"[\s\S]*Goto compare_version/);
  assert.match(nsis, /nsis_tauri_utils::SemverCompare "\$\{VERSION\}" \$R0/);
});

test("WiX restores and rewrites the original registry and directory identities", () => {
  assert.match(section("wix"), /template = "crates\/patchfleet-desktop\/assets\/windows\/main.wxs"/);
  assert.match(wix, /RegistrySearch Id="PrevInstallDirReg" Root="HKCU" Key="Software\\\\Patchfleet\\\\Patchfleet" Name="InstallDir"/);
  assert.match(wix, /RegistryKey Root="HKCU" Key="Software\\\\Patchfleet\\\\Patchfleet"/);
  assert.doesNotMatch(wix, /Key="Software\\\\\{\{manufacturer\}\}\\\\\{\{product_name\}\}"/);
  assert.match(wix, /Directory Id="INSTALLDIR" Name="Patchfleet"/);
  assert.match(wix, /Directory Id="ApplicationProgramsFolder" Name="Patchfleet"/);
  assert.match(wix, /UpgradeCode="\{\{upgrade_code\}\}"/);
  assert.match(wix, /ShortcutProperty Key="System.AppUserModel.ID" Value="\{\{identifier\}\}"/);
  assert.match(wix, /Name="\{\{product_name\}\}"[\s\S]*Manufacturer="\{\{manufacturer\}\}"/);
});

test("legacy shortcut cleanup requires prior product metadata and never includes canonical data", () => {
  const cleanup = nsis.match(/Function RemoveLegacyShortcuts([\s\S]*?)FunctionEnd/)[1];
  assert.match(cleanup, /ReadRegStr .*"\$\{UNINSTKEY\}" "UninstallString"/);
  assert.match(cleanup, /ReadRegStr .*"\$\{MANUPRODUCTKEY\}" "InstallDir"/);
  assert.match(cleanup, /\$\{If\} "\$0\$1" != ""[\s\S]*Delete "\$DESKTOP/);
  assert.doesNotMatch(cleanup, /(?:RMDir|\.json|\.patchfleet|APPDATA)/i);
  assert.match(wix, /Property Id="LEGACYINSTALLDIR"[\s\S]*RegistrySearch Id="LegacyShortcutInstallReg"/);
  for (const property of ["LEGACYDESKTOPDIR", "LEGACYMENUDIR"]) {
    assert.match(wix, new RegExp(`SetProperty Id="${property}"[^>]*>LEGACYINSTALLDIR<`));
    assert.match(wix, new RegExp(`RemoveFile[^>]*Name="Patchfleet\\.lnk" Property="${property}"`));
  }
  assert.doesNotMatch(section("nsis").replace(/^#.*$/gm, ""), /appdata[-A-Za-z]*\s*=/);
  assert.match(config, /identifier = "com.patchfleet.desktop"/);
  assert.match(config, /path = "patchfleet-desktop"/);
});
