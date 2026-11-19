import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

const directory = "crates/moxcrew-desktop/assets/windows/";
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
  return Object.fromEntries(["UNINSTKEY", "MANUPRODUCTKEY", "INSTALLIDENTITY", "LEGACYUNINSTKEY", "LEGACYMANUPRODUCTKEY"].map((key) => [key, expand(result[key])]));
}

test("NSIS installs under Moxcrew and recognizes the exact prior identity", () => {
  assert.match(section("nsis"), /template = "crates\/moxcrew-desktop\/assets\/windows\/installer.nsi"/);
  assert.match(section("nsis"), /installMode = "currentUser"/);
  assert.deepEqual(nsisDefines("Moxcrew", "Moxcrew"), {
    UNINSTKEY: "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Moxcrew",
    MANUPRODUCTKEY: "Software\\Moxcrew\\Moxcrew",
    INSTALLIDENTITY: "Moxcrew",
    LEGACYUNINSTKEY: "Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\Patchfleet",
    LEGACYMANUPRODUCTKEY: "Software\\Patchfleet\\Patchfleet",
  });
  assert.match(nsis, /StrCpy \$INSTDIR "\$LOCALAPPDATA\\\$\{INSTALLIDENTITY\}"/);
  assert.doesNotMatch(nsis, /\$(?:LOCALAPPDATA|PROGRAMFILES64|PROGRAMFILES)\\\$\{PRODUCTNAME\}/);
  assert.match(nsis, /MUI_STARTMENUPAGE_DEFAULTFOLDER "\$\{INSTALLIDENTITY\}"/);
  assert.match(nsis, /"DisplayName" "\$\{PRODUCTNAME\}"/);
  assert.match(nsis, /"Publisher" "\$\{MANUFACTURER\}"/);
  assert.match(nsis, /ApplicationID::Set .*"\$\{IDENTIFIER\}"/);
});

test("NSIS detects old WiX and NSIS installations before copying the new executable", () => {
  assert.match(nsis, /\$R0 == "\$\{PRODUCTNAME\}"[\s\S]*\$R1 == "\$\{MANUFACTURER\}"[\s\S]*Goto wix_match/);
  assert.match(nsis, /\$R0 != "\$\{LEGACYIDENTITY\}"[\s\S]*\$R1 != "\$\{LEGACYMANUFACTURER\}"[\s\S]*Goto wix_loop/);
  assert.match(nsis, /wix_match:[\s\S]*StrCpy \$R7 "wix"[\s\S]*Goto compare_version/);
  assert.match(nsis, /ReadRegStr \$R1 SHCTX "\$\{LEGACYUNINSTKEY\}" "UninstallString"/);
  assert.match(nsis, /\$R0\$R1" != ""[\s\S]*\$R2 != ""[\s\S]*Abort/);
  assert.match(nsis, /\$R7 == "legacy-nsis"[\s\S]*StrCpy \$R5 "1"/);
  assert.match(nsis, /IfSilent 0 legacy_silent_check_done[\s\S]*ReadRegStr \$R0 SHCTX "\$\{LEGACYUNINSTKEY\}" "UninstallString"[\s\S]*Abort/);
  assert.match(nsis, /nsis_tauri_utils::SemverCompare "\$\{VERSION\}" \$R0/);
  assert.match(nsis, /ReadRegStr \$4 SHCTX "\$\{LEGACYMANUPRODUCTKEY\}" ""[\s\S]*ExecWait '\$R1 \/P _\?=\$4'[\s\S]*RMDir \$4/);
});

test("WiX writes Moxcrew identities and reads only the old shortcut location", () => {
  assert.match(section("wix"), /template = "crates\/moxcrew-desktop\/assets\/windows\/main.wxs"/);
  assert.match(wix, /RegistrySearch Id="PrevInstallDirReg" Root="HKCU" Key="Software\\Moxcrew\\Moxcrew" Name="InstallDir"/);
  assert.match(wix, /RegistryKey Root="HKCU" Key="Software\\Moxcrew\\Moxcrew"/);
  assert.doesNotMatch(wix, /Key="[^"]*\\\\/);
  assert.doesNotMatch(wix, /Key="Software\\\\\{\{manufacturer\}\}\\\\\{\{product_name\}\}"/);
  assert.match(wix, /Directory Id="INSTALLDIR" Name="Moxcrew"/);
  assert.match(wix, /Directory Id="ApplicationProgramsFolder" Name="Moxcrew"/);
  assert.match(wix, /UpgradeCode="c1c93449-0cd9-535c-908c-ce6cf4d9d365"/);
  assert.match(wix, /ShortcutProperty Key="System.AppUserModel.ID" Value="\{\{identifier\}\}"/);
  assert.match(wix, /Name="\{\{product_name\}\}"[\s\S]*Manufacturer="\{\{manufacturer\}\}"/);
});

test("legacy shortcut cleanup requires prior product metadata and never includes canonical data", () => {
  const cleanup = nsis.match(/Function RemoveLegacyShortcuts([\s\S]*?)FunctionEnd/)[1];
  assert.match(cleanup, /ReadRegStr .*"\$\{LEGACYUNINSTKEY\}" "UninstallString"/);
  assert.match(cleanup, /ReadRegStr .*"\$\{LEGACYMANUPRODUCTKEY\}" "InstallDir"/);
  assert.match(cleanup, /\$\{If\} "\$0\$1" != ""[\s\S]*Delete "\$DESKTOP/);
  assert.doesNotMatch(cleanup, /(?:RMDir|\.json|\.patchfleet|APPDATA)/i);
  assert.match(wix, /Property Id="LEGACYINSTALLDIR"[\s\S]*RegistrySearch Id="LegacyShortcutInstallReg"/);
  for (const property of ["LEGACYDESKTOPDIR", "LEGACYMENUDIR"]) {
    assert.match(wix, new RegExp(`SetProperty Id="${property}"[^>]*>LEGACYINSTALLDIR<`));
    assert.match(wix, new RegExp(`RemoveFile[^>]*Name="Patchfleet\\.lnk" Property="${property}"`));
  }
  assert.doesNotMatch(section("nsis").replace(/^#.*$/gm, ""), /appdata[-A-Za-z]*\s*=/);
  assert.match(config, /identifier = "com.moxcrew.desktop"/);
  assert.match(config, /path = "moxcrew-desktop"/);
});
