#!/bin/deno

interface QPM1 {
  version: string;
  sharedDir: string;
  dependenciesDir: string;
  info: {
    name?: string;
    id: string;
    version: string;

    url?: string;
    author?: string;

    additionalData?: {
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
  workspace: {
    scripts:
      | {
          build?: string[];
          debug?: string[];
          copy?: string[];
          qmod?: string[];
        }
      | Record<string, string[]>;
    qmodIncludeDirs: string[];
    qmodIncludeFiles: string[];
    qmodOutput: string;
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

type QPM2Triplet = {
  dependencies: Record<string, QPM2Dep>;
  devDependencies: Record<string, QPM2Dep>;
  env: Record<string, string>;

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

  // QMod URL for this triplet
  qmodUrl?: string;

  // QMod ID for this triplet
  qmodId?: string;

  // QMod template path for this triplet (e.g. mod.template.json)
  qmodTemplate?: string;

  // Output binaries for this triplet
  outBinaries?: string[];
};

interface QPM2 {
  id: string;
  version: string;
  dependenciesDirectory: string;
  sharedDirectory: string;
  workspace: {
    scripts: Record<string, string[] | undefined>;
    qmodIncludeDirs: string[];
    qmodIncludeFiles: string[];
  };
  additionalData?: {
    description?: string;
    author?: string;
    license?: string;
  };
  triplets: {
    default: QPM2Triplet;
    [key: string]: QPM2Triplet;
  };
  configVersion: string;
  toolchainOut: string;
}

interface QPM2Dep {
  versionRange: string;
  triplet?: string | "default" | null;
  qmodExport?: boolean;
  qmodRequired?: boolean;
}

function convertDep(dep: QPM1Dep): QPM2Dep {
  return {
    versionRange: dep.versionRange,
    triplet: "default",
    qmodExport: dep.additionalData.includeQmod ?? false,
    qmodRequired: dep.additionalData.includeQmod ?? false,
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
  "./build/" +
  (qpm1.info.additionalData?.overrideSoName ?? `${qpm1.info.id}.so`);

const qpm2: QPM2 = {
  id: qpm1.info.id,
  version: qpm1.info.version,
  dependenciesDirectory: qpm1.dependenciesDir,
  sharedDirectory: qpm1.sharedDir,
  workspace: {
    scripts: {
      build: qpm1.workspace.scripts.build,
      debug: qpm1.workspace.scripts.debug,
      copy: qpm1.workspace.scripts.copy,
      qmod: qpm1.workspace.scripts.qmod,
    },
    qmodIncludeDirs: qpm1.workspace.qmodIncludeDirs,
    qmodIncludeFiles: qpm1.workspace.qmodIncludeFiles,
  },
  additionalData: {
    description: "",
    author: qpm1.info.author,
    license: "",
  },
  triplets: {
    default: {
      dependencies: dependencies,
      devDependencies: devDependencies,
      compileOptions: qpm1.info.additionalData?.compileOptions,
      outBinaries: [qpm2Bin],
      qmodId: qpm1.info.id,
      qmodTemplate: "mod.template.json",
      qmodUrl: qpm1.info.additionalData?.modLink,

      env: {},
    },
  },
  configVersion: "2.0.0",
  toolchainOut: qpm1.info.additionalData?.toolchainOut ?? "toolchain.json",
};

Deno.writeTextFile("./qpm2.json", JSON.stringify(qpm2, undefined, 2));
