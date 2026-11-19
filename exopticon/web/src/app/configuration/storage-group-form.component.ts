import { Component, OnInit, input, output } from "@angular/core";
import { FormsModule } from "@angular/forms";
import { ResourceMetadata } from "../camera";
import {
  ConfigurationFieldError,
  ConfigurationForm,
  FieldErrors,
  MetadataPatch,
  numberError,
} from "./configuration-form";

export interface StorageGroup {
  metadata: ResourceMetadata;
  spec: { storagePath: string; maxStorageSize: number };
}

@Component({
  selector: "app-storage-group-configuration-form",
  imports: [FormsModule, ConfigurationForm, ConfigurationFieldError],
  templateUrl: "./storage-group-form.component.html",
})
export class StorageGroupFormComponent implements OnInit {
  readonly group = input.required<StorageGroup | null>();
  readonly listLink = input.required<string[]>();
  readonly errors = input.required<FieldErrors>();
  readonly saving = input.required<boolean>();
  readonly saved = output<object>();
  storagePath = "";
  maxStorageSize: number | null = null;
  pathError = "";
  quotaError = "";

  ngOnInit(): void {
    const spec = this.group()?.spec;
    this.storagePath = spec?.storagePath ?? "";
    this.maxStorageSize = spec?.maxStorageSize ?? null;
  }

  save(metadata: MetadataPatch | null): void {
    const existing = this.group();
    const quotaChanged =
      !existing || this.maxStorageSize !== existing.spec.maxStorageSize;
    this.pathError = !existing && !this.storagePath ? "Enter a value." : "";
    this.quotaError = quotaChanged
      ? numberError(this.maxStorageSize, 0, 8796093022207)
      : "";
    if (!metadata || this.pathError || this.quotaError) {
      return;
    }
    this.saved.emit({
      metadata,
      spec: {
        ...(!existing ? { storagePath: this.storagePath } : {}),
        ...(quotaChanged ? { maxStorageSize: this.maxStorageSize } : {}),
      },
    });
  }
}
