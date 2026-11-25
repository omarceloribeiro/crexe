const platforms = ['windows', 'macos', 'linux'];

export function httpsUrl(value) {
  if (typeof value !== 'string') return null;
  try {
    const url = new URL(value);
    return url.protocol === 'https:' && !url.username && !url.password ? url.href : null;
  } catch { return null; }
}

// A partial release enables only its complete, usable platform entries.
export function availablePackages(manifest) {
  if (typeof manifest?.version !== 'string' || !manifest.version.trim() || !httpsUrl(manifest.releaseUrl)) {
    return new Set();
  }
  return new Set(platforms.filter((os) => {
    const entry = manifest.platforms?.[os];
    return httpsUrl(entry?.url) && httpsUrl(entry?.checksumUrl);
  }));
}

export function quickstartFor(os, nativePackage, english) {
  if (!platforms.includes(os)) throw new RangeError(`Unknown platform: ${os}`);
  const executable = os === 'windows'
    ? (nativePackage ? '.\\crexe.exe' : '.\\target\\release\\crexe.exe')
    : (nativePackage ? './crexe' : './target/release/crexe');
  const requirements = english ? {
    windows: 'Windows: .NET 8 SDK for Windows Forms.',
    macos: 'macOS: Apple Clang and the Cocoa SDK for the experimental native profile. Configuration and the editor require a graphical session.',
    linux: 'Linux: GCC, Make, pkg-config and GTK3 development libraries for the experimental native profile. Configuration and the editor require a graphical session.',
  } : {
    windows: 'Windows: .NET 8 SDK para Windows Forms.',
    macos: 'macOS: Apple Clang e SDK Cocoa para o perfil nativo experimental. Configuração e editor precisam de uma sessão gráfica.',
    linux: 'Linux: GCC, Make, pkg-config e bibliotecas de desenvolvimento GTK3 para o perfil nativo experimental. Configuração e editor precisam de uma sessão gráfica.',
  };
  const buildRequirements = english ? {
    windows: ' To build the engine from source, install Rust/Cargo and the Visual C++ linker.',
    macos: ' To build the engine from source, install Rust/Cargo.',
    linux: ' To build the engine from source, install Rust/Cargo and the desktop libraries required by the build.',
  } : {
    windows: ' Para compilar a engine a partir dos fontes, prepare Rust/Cargo e o linker Visual C++.',
    macos: ' Para compilar a engine a partir dos fontes, prepare Rust/Cargo.',
    linux: ' Para compilar a engine a partir dos fontes, prepare Rust/Cargo e as bibliotecas de desktop exigidas pelo build.',
  };
  return {
    requirements: requirements[os] + (nativePackage ? '' : buildRequirements[os]),
    firstTitle: english
      ? (nativePackage ? 'Extract and check the engine' : 'Build the engine from source')
      : (nativePackage ? 'Extraia e verifique a engine' : 'Compile os fontes da engine'),
    firstDescription: english
      ? (nativePackage ? 'Download the package for this system, verify its SHA-256 and extract it. In the extracted folder, check the engine version:' : 'Extract the source ZIP. In the extracted folder, run:')
      : (nativePackage ? 'Baixe o pacote deste sistema, confira o SHA-256 e extraia. Na pasta extraída, verifique a versão da engine:' : 'Extraia o ZIP do código-fonte. Na pasta extraída, execute:'),
    firstCommand: nativePackage ? `${executable} --version` : 'cargo build --release --locked',
    firstCopyLabel: english
      ? (nativePackage ? 'Copy version check command' : 'Copy build command')
      : (nativePackage ? 'Copiar comando de verificação da versão' : 'Copiar comando de compilação'),
    configureCommand: `${executable} configure\n${executable} doctor --check`,
    runDescription: english
      ? `Use the example already included in the ${nativePackage ? 'package' : 'source code'}. This command starts generation, compilation and execution.`
      : `Use o exemplo já incluído no ${nativePackage ? 'pacote' : 'código-fonte'}. Este comando inicia a geração, compilação e execução.`,
    runCommand: `${executable} ${nativePackage ? 'examples/calculadora.crexe' : 'examples/plaintext/calculadora.crexe'}`,
  };
}
