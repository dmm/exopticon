import { DOCUMENT } from "@angular/common";
import { DestroyRef, Injectable, inject } from "@angular/core";
import { takeUntilDestroyed } from "@angular/core/rxjs-interop";
import { SwUpdate } from "@angular/service-worker";

@Injectable({ providedIn: "root" })
export class PwaUpdateService {
  private readonly updates = inject(SwUpdate);
  private readonly document = inject(DOCUMENT);
  private readonly destroyRef = inject(DestroyRef);
  private checking = false;

  start(): void {
    if (!this.updates.isEnabled) return;

    this.updates.versionUpdates
      .pipe(takeUntilDestroyed(this.destroyRef))
      .subscribe((event) => {
        switch (event.type) {
          case "VERSION_DETECTED":
            console.info("Downloading PWA update:", event.version.hash);
            break;
          case "VERSION_READY":
            console.info("PWA update ready:", {
              currentVersion: event.currentVersion.hash,
              latestVersion: event.latestVersion.hash,
            });
            break;
          case "VERSION_INSTALLATION_FAILED":
            console.error(
              "Failed to install PWA update:",
              event.version.hash,
              event.error,
            );
            break;
          case "NO_NEW_VERSION_DETECTED":
            console.debug("PWA is up to date:", event.version.hash);
            break;
        }
      });

    const onVisibilityChange = () => {
      if (this.document.visibilityState === "visible") {
        void this.checkForUpdate();
      }
    };

    this.document.addEventListener("visibilitychange", onVisibilityChange);
    this.destroyRef.onDestroy(() => {
      this.document.removeEventListener("visibilitychange", onVisibilityChange);
    });

    // The worker registers immediately; checking must not block app startup.
    void this.checkForUpdate();
  }

  private async checkForUpdate(): Promise<void> {
    if (this.checking) return;

    this.checking = true;
    try {
      await this.updates.checkForUpdate();
    } catch (error) {
      console.warn("Failed to check for PWA updates:", error);
    } finally {
      this.checking = false;
    }
  }
}
