import { HttpClient, HttpErrorResponse } from "@angular/common/http";
import {
  ChangeDetectorRef,
  Component,
  DestroyRef,
  inject,
  OnInit,
} from "@angular/core";
import { takeUntilDestroyed } from "@angular/core/rxjs-interop";
import { ActivatedRoute, Router, RouterLink } from "@angular/router";
import { EMPTY, Observable, forkJoin, of } from "rxjs";
import { catchError, defaultIfEmpty, map, switchMap } from "rxjs/operators";

import { Camera, ResourceMetadata } from "../camera";
import { CameraGroup } from "../camera-group";
import { CameraFormComponent } from "./camera-form.component";
import { CameraGroupFormComponent } from "./camera-group-form.component";
import { FieldErrors } from "./configuration-form";
import {
  StorageGroup,
  StorageGroupFormComponent,
} from "./storage-group-form.component";
import { ConfigurationUser, UserFormComponent } from "./user-form.component";

const resources = {
  storage_groups: { label: "Storage groups", singular: "storage group" },
  cameras: { label: "Cameras", singular: "camera" },
  camera_groups: { label: "Camera groups", singular: "camera group" },
  users: { label: "Users", singular: "user" },
};

type Resource = keyof typeof resources;
interface NamedResource {
  metadata: ResourceMetadata;
  status: { phase?: string };
}

type ResourceRow = NamedResource | ConfigurationUser;

interface Item extends ResourceMetadata {
  phase?: string;
}

type ResourceForm =
  | {
      resource: "cameras";
      camera: Camera | null;
      storageGroups: ResourceMetadata[];
    }
  | {
      resource: "camera_groups";
      group: CameraGroup | null;
      cameras: ResourceMetadata[];
    }
  | { resource: "storage_groups"; group: StorageGroup | null }
  | { resource: "users"; user: ConfigurationUser | null };

interface PageData {
  items?: Item[];
  displayName?: string;
  form?: ResourceForm;
}

@Component({
  selector: "app-configuration",
  imports: [
    RouterLink,
    CameraFormComponent,
    CameraGroupFormComponent,
    StorageGroupFormComponent,
    UserFormComponent,
  ],
  templateUrl: "./configuration.component.html",
  styleUrl: "./configuration.component.css",
})
export class ConfigurationComponent implements OnInit {
  private readonly http = inject(HttpClient);
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly changeDetector = inject(ChangeDetectorRef);
  private readonly destroyRef = inject(DestroyRef);
  private routeVersion = 0;

  readonly resources = resources;
  readonly resourceKeys = Object.keys(resources) as Resource[];
  resource: Resource = "storage_groups";
  mode: "list" | "new" | "edit" | "delete" = "list";
  name = "";
  displayName = "";
  items: Item[] = [];
  form: ResourceForm | null = null;
  loading = true;
  loadFailed = false;
  saving = false;
  message = "";
  errors: FieldErrors = {};

  get listLink(): string[] {
    return ["/configuration", this.resource];
  }

  private get url(): string {
    return `/v1/${this.resource}`;
  }

  private get itemUrl(): string {
    return `${this.url}/${encodeURIComponent(this.name)}`;
  }

  ngOnInit(): void {
    this.route.paramMap
      .pipe(
        switchMap((params) => {
          this.routeVersion += 1;
          const resource = params.get("resource");
          if (!resource || !this.resourceKeys.includes(resource as Resource)) {
            void this.router.navigate(["/configuration"]);
            return EMPTY;
          }
          this.resource = resource as Resource;
          this.mode = this.route.snapshot.data["mode"];
          this.name = params.get("name") ?? "";
          this.loading = true;
          this.loadFailed = false;
          this.message = "";
          this.errors = {};
          this.changeDetector.markForCheck();

          return this.readPage().pipe(
            catchError((error) => {
              this.showError(error);
              return of(null);
            }),
            defaultIfEmpty<PageData | null>(null),
          );
        }),
        takeUntilDestroyed(this.destroyRef),
      )
      .subscribe((page) => {
        this.displayName = page?.displayName ?? "";
        this.items = page?.items ?? [];
        this.form = page?.form ?? null;
        this.loadFailed = page === null;
        this.loading = false;
        this.changeDetector.markForCheck();
      });
  }

  private readPage(): Observable<PageData> {
    if (this.mode === "list") {
      return this.http.get<ResourceRow[]>(this.url).pipe(
        map((rows) => ({
          items: rows.map((row) =>
            "metadata" in row
              ? { ...row.metadata, phase: row.status.phase }
              : { name: row.username, displayName: row.displayName },
          ),
        })),
      );
    }

    if (this.mode === "delete") {
      return this.http.get<ResourceRow>(this.itemUrl).pipe(
        map((row) => ({
          displayName:
            "metadata" in row ? row.metadata.displayName : row.displayName,
        })),
      );
    }

    return this.readForm(this.resource, this.mode === "new", this.itemUrl).pipe(
      map((form) => ({ form })),
    );
  }

  private readForm(
    resource: Resource,
    creating: boolean,
    itemUrl: string,
  ): Observable<ResourceForm> {
    switch (resource) {
      case "cameras":
        return creating
          ? this.http.get<StorageGroup[]>("/v1/storage_groups").pipe(
              map((groups) => ({
                resource,
                camera: null,
                storageGroups: groups.map((group) => group.metadata),
              })),
            )
          : this.http
              .get<Camera>(itemUrl)
              .pipe(map((camera) => ({ resource, camera, storageGroups: [] })));
      case "camera_groups":
        return forkJoin([
          creating ? of(null) : this.http.get<CameraGroup>(itemUrl),
          this.http.get<Camera[]>("/v1/cameras"),
        ]).pipe(
          map(([group, cameras]) => ({
            resource,
            group,
            cameras: cameras.map((camera) => camera.metadata),
          })),
        );
      case "storage_groups":
        return creating
          ? of({ resource, group: null })
          : this.http
              .get<StorageGroup>(itemUrl)
              .pipe(map((group) => ({ resource, group })));
      case "users":
        return creating
          ? of({ resource, user: null })
          : this.http
              .get<ConfigurationUser>(itemUrl)
              .pipe(map((user) => ({ resource, user })));
    }
  }

  save(body: object): Promise<void> {
    return this.submit(() =>
      this.mode === "new"
        ? this.http.post(this.url, body)
        : this.http.patch(this.itemUrl, body),
    );
  }

  deleteResource(): Promise<void> {
    return this.submit(() => this.http.delete(this.itemUrl));
  }

  private async submit(request: () => Observable<unknown>): Promise<void> {
    if (this.saving) {
      return;
    }
    this.errors = {};
    this.message = "";
    this.saving = true;
    const routeVersion = this.routeVersion;
    const listLink = this.listLink;
    try {
      await request().toPromise();
      if (routeVersion === this.routeVersion && !this.destroyRef.destroyed) {
        await this.router.navigate(listLink);
      }
    } catch (error) {
      if (routeVersion === this.routeVersion && !this.destroyRef.destroyed) {
        this.showError(error);
      }
    } finally {
      this.saving = false;
      if (!this.destroyRef.destroyed) {
        this.changeDetector.markForCheck();
      }
    }
  }

  private showError(error: unknown): void {
    if (error instanceof HttpErrorResponse) {
      const data = error.error;
      if (
        typeof data?.field === "string" &&
        typeof data?.message === "string"
      ) {
        this.errors[data.field] = data.message;
        return;
      }
      if (typeof data?.message === "string") {
        this.message = data.message;
        return;
      }
      if (error.status === 404) {
        this.message = "This resource is no longer available.";
        return;
      }
    }
    this.message = "The request failed. Reload the page and try again.";
  }
}
