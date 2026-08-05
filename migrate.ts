#!/bin/deno

interface QPM1 {
  version?: string;
  sharedDir: string;
  dependenciesDir: string;
  info: {
    name?: string;
    id: string;
    version: string;

    url?: string;
    author?: string;

    additionalData?: {
      headersOnly?: boolean,
      overrideSoName?: string;
      cmake?: boolean;
      toolchainOut?: string;
      modLink?: string;
      compileOptions?: {
        // Additional include paths to add, relative to the extern directory.
        includePaths?: string[];

        // Additional system include paths to add, relative to the extern directory.
        systemIncludes?: string[];

        // Additional C++ flags to add.
        cppFlags?: string[];

        // Additional C flags to add.
        cFlags?: string[];
      };
    };
  };
  workspace?: {
    // qpm1's scripts map is a plain BTreeMap<String, Vec<String>> - any key allowed
    scripts?: Record<string, string[]>;
    ndk?: string;
    qmodIncludeDirs?: string[];
    qmodIncludeFiles?: string[];
    qmodOutput?: string;
  };
  dependencies: Array<QPM1Dep>;
}

type QPM1Dep = {
  id: string;
  versionRange: string;
  additionalData: {
    includeQmod?: boolean;
    required?: boolean;
    private?: boolean;
  };
};

interface QPM2 {
  configVersion: string;
  id: string;
  version: string;
  additionalData?: {
    description?: string;
    author?: string;
    license?: string;
    url?: string;
  };
  workspace?: {
    scripts?: Record<string, string[] | undefined>;
    env?: Record<string, string>;
    ndk?: string;
    outBinaries?: string[];
    toolchainOut?: string;
    cmake?: boolean;
  };
  qmod?: {
    output?: string;
    template?: string;
    searchDirs?: string[];
    includeFiles?: string[];
    downloadUrl?: string;
    id?: string;
  };
  dependencies: Record<string, QPM2Dep>;
  devDependencies: Record<string, QPM2Dep>;
  dependenciesDirectory: string;
  sharedDirectory: string;

  // Additional compile options for the package
  compileOptions?: {
    // Additional include paths to add, relative to the extern directory.
    includePaths?: string[];

    // Additional system include paths to add, relative to the extern directory.
    systemIncludes?: string[];

    // Additional C++ flags to add.
    cppFlags?: string[];

    // Additional C flags to add.
    cFlags?: string[];
  };
}

interface QPM2Dep {
  versionRange: string;
  qmod?: "none" | "required" | "optional";
  qpkgUrl?: string;
}

function convertDep(dep: QPM1Dep): QPM2Dep {
  // Both default to true when omitted (see QPM.Package's PackageDependencyModifier).
  const includeQmod = dep.additionalData.includeQmod ?? true;
  const required = dep.additionalData.required ?? true;

  let qmod: QPM2Dep["qmod"] = "required";
  if (!includeQmod) {
    qmod = "none";
  } else if (!required) {
    qmod = "optional";
  }

  return {
    versionRange: dep.versionRange,
    qmod,
  };
}

const qpm1: QPM1 = JSON.parse(await Deno.readTextFile("./qpm.json"));

const dependencies = qpm1.dependencies
  .filter((x) => !x.additionalData.private)
  .reduce((acc, dep) => {
    acc[dep.id] = convertDep(dep);
    return acc;
  }, {} as Record<string, QPM2Dep>);

const devDependencies = qpm1.dependencies
  .filter((x) => x.additionalData.private)
  .reduce((acc, dep) => {
    acc[dep.id] = convertDep(dep);
    return acc;
  }, {} as Record<string, QPM2Dep>);

const qpm2Bin =
  qpm1.info.additionalData?.headersOnly ? null :
    (qpm1.info.additionalData?.overrideSoName ?? `lib${qpm1.info.id}.so`);
const outBinaries = qpm2Bin ? [qpm2Bin] : [];

const qpm2: QPM2 = {
  configVersion: "2.0.0",
  id: qpm1.info.id,
  version: qpm1.info.version,
  additionalData: {
    description: "",
    author: qpm1.info.author,
    license: "",
    url: qpm1.info.url,
  },
  workspace: {
    // qpm1's scripts map allows arbitrary keys, not just build/debug/copy/qmod
    scripts: qpm1.workspace?.scripts,
    ndk: qpm1.workspace?.ndk,
    outBinaries,
    toolchainOut: qpm1.info.additionalData?.toolchainOut ?? "toolchain.json",
    cmake: qpm1.info.additionalData?.cmake,
  },
  qmod: {
    id: qpm1.info.id,
    template: "mod.template.json",
    output: qpm1.workspace?.qmodOutput,
    downloadUrl: qpm1.info.additionalData?.modLink,
    searchDirs: qpm1.workspace?.qmodIncludeDirs,
    includeFiles: qpm1.workspace?.qmodIncludeFiles,
  },
  dependencies: dependencies,
  devDependencies: devDependencies,
  dependenciesDirectory: qpm1.dependenciesDir,
  sharedDirectory: qpm1.sharedDir,

  compileOptions: qpm1.info.additionalData?.compileOptions,
};

Deno.writeTextFile("./qpm2.json", JSON.stringify(qpm2, undefined, 2));
