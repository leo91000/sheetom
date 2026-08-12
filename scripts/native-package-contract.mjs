export function nativePlatformMetadata(target) {
  const [platform, architecture, abi] = target.target.split("-");
  if (platform === "darwin") return { os: ["darwin"], cpu: [architecture] };
  if (platform === "win32") return { os: ["win32"], cpu: [architecture] };
  const cpu = architecture === "arm" ? "arm" : architecture;
  const libc = abi.includes("musl") ? "musl" : "glibc";
  return { os: ["linux"], cpu: [cpu], libc: [libc] };
}

export function nativePackageManifest(rootManifest, target, { publishable = false } = {}) {
  const manifest = {
    name: target.packageName,
    version: rootManifest.version,
    description: `Native SheetOM engine for ${target.target}`,
    license: rootManifest.license,
    repository: {
      ...rootManifest.repository,
      directory: `packages/native-${target.target}`,
    },
    type: "commonjs",
    main: "./index.cjs",
    files: ["index.cjs", target.artifact, "LICENSE", "README.md"],
    preferUnplugged: true,
    engines: rootManifest.engines,
    publishConfig: { access: "public" },
  };
  return publishable ? { ...manifest, ...nativePlatformMetadata(target) } : manifest;
}
