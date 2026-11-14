import { DOCUMENT } from "@angular/common";
import { DestroyRef, Injectable, inject } from "@angular/core";
import { SwUpdate } from "@angular/service-worker";

@Injectable({ providedIn: "root" })
export class PwaUpdateService {
  private readonly updates = inject(SwUpdate);
  private readonly document = inject(DOCUMENT);
  private readonly destroyRef = inject(DestroyRef);
  private checking = false;

  start(): void {
    if (!this.updates.isEnabled) return;

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
