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
    scripts?:
      | {
          build?: string[];
          debug?: string[];
          copy?: string[];
          qmod?: string[];
        }
      | Record<string, string[]>;
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
}

function convertDep(dep: QPM1Dep): QPM2Dep {
  return {
    versionRange: dep.versionRange,
    qmod: dep.additionalData.includeQmod ? "required" : "none",
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
    scripts: {
      build: qpm1.workspace?.scripts?.build,
      debug: qpm1.workspace?.scripts?.debug,
      copy: qpm1.workspace?.scripts?.copy,
      qmod: qpm1.workspace?.scripts?.qmod,
    },
    outBinaries,
    toolchainOut: qpm1.info.additionalData?.toolchainOut ?? "toolchain.json",
    cmake: qpm1.info.additionalData?.cmake,
  },
  qmod: {
    id: qpm1.info.id,
    template: "mod.template.json",
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
