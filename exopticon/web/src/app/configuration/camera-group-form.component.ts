import { Component, OnInit, input, output } from "@angular/core";
import { ResourceMetadata } from "../camera";
import { ALL_GROUP_NAME, CameraGroup } from "../camera-group";
import {
  ConfigurationFieldError,
  ConfigurationForm,
  FieldErrors,
  MetadataPatch,
} from "./configuration-form";

@Component({
  selector: "app-camera-group-configuration-form",
  imports: [ConfigurationForm, ConfigurationFieldError],
  templateUrl: "./camera-group-form.component.html",
})
export class CameraGroupFormComponent implements OnInit {
  readonly group = input.required<CameraGroup | null>();
  readonly cameras = input.required<ResourceMetadata[]>();
  readonly listLink = input.required<string[]>();
  readonly errors = input.required<FieldErrors>();
  readonly saving = input.required<boolean>();
  readonly saved = output<object>();
  members: string[] = [];

  get includesAllCameras(): boolean {
    return this.group()?.metadata.name === ALL_GROUP_NAME;
  }

  get availableCameras(): ResourceMetadata[] {
    return this.cameras().filter(
      (camera) => !this.members.includes(camera.name),
    );
  }

  ngOnInit(): void {
    this.members = [...(this.group()?.spec.members ?? [])];
  }

  moveMember(index: number, offset: number): void {
    const next = index + offset;
    [this.members[index], this.members[next]] = [
      this.members[next],
      this.members[index],
    ];
  }

  addMember(select: HTMLSelectElement): void {
    if (select.value) {
      this.members.push(select.value);
      select.value = "";
    }
  }

  save(metadata: MetadataPatch | null): void {
    if (!metadata) {
      return;
    }
    const previous = this.group()?.spec.members;
    const membersChanged =
      !previous ||
      this.members.length !== previous.length ||
      this.members.some((name, index) => name !== previous[index]);
    this.saved.emit({
      metadata,
      spec: membersChanged ? { members: this.members } : {},
    });
  }
}
