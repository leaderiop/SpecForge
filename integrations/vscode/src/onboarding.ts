import * as vscode from "vscode";

const ONBOARDING_SHOWN_KEY = "specforge.onboardingShown";
const WALKTHROUGH_ID = "specforge.specforge#specforge.getStarted";

/**
 * Register onboarding experience for first-time users.
 *
 * On first activation when no specforge.json is present, shows a
 * notification offering to open the Getting Started walkthrough or
 * initialize a new project.
 */
export function registerOnboarding(context: vscode.ExtensionContext): void {
  const alreadyShown = context.globalState.get<boolean>(ONBOARDING_SHOWN_KEY, false);
  if (alreadyShown) {
    return;
  }

  // Mark as shown immediately so we never show twice, even if the user
  // dismisses the notification without clicking a button.
  context.globalState.update(ONBOARDING_SHOWN_KEY, true);

  // Only show the welcome notification if there is no existing project.
  hasSpecforgeProject().then((hasProject) => {
    if (hasProject) {
      return;
    }

    vscode.window
      .showInformationMessage(
        "Welcome to SpecForge! Write structured specs that build typed entity graphs for AI agents.",
        "Get Started",
        "Initialize Project"
      )
      .then((selection) => {
        if (selection === "Get Started") {
          vscode.commands.executeCommand(
            "workbench.action.openWalkthrough",
            WALKTHROUGH_ID,
            false
          );
        } else if (selection === "Initialize Project") {
          vscode.commands.executeCommand("specforge.init");
        }
      });
  });
}

async function hasSpecforgeProject(): Promise<boolean> {
  const folders = vscode.workspace.workspaceFolders;
  if (!folders) {
    return false;
  }

  for (const folder of folders) {
    const configUri = vscode.Uri.joinPath(folder.uri, "specforge.json");
    try {
      await vscode.workspace.fs.stat(configUri);
      return true;
    } catch {
      // specforge.json not found in this folder, continue checking
    }
  }

  return false;
}
