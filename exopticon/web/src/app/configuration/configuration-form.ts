import {
  Component,
  ElementRef,
  ViewEncapsulation,
  input,
  output,
  viewChild,
} from "@angular/core";
import { FormsModule } from "@angular/forms";
import { RouterLink } from "@angular/router";
import { ResourceMetadata } from "../camera";

export type FieldErrors = Record<string, string>;
export type MetadataPatch = Partial<ResourceMetadata>;

export function numberError(
  value: number | null,
  min: number,
  max: number,
): string {
  if (value === null) {
    return "Enter a value.";
  }
  return Number.isInteger(value) && value >= min && value <= max
    ? ""
    : `Enter a whole number from ${min} to ${max}.`;
}

@Component({
  selector: "app-configuration-field-error",
  template:
    '@if (message()) { <span class="error" role="alert">{{ message() }}</span> }',
})
export class ConfigurationFieldError {
  readonly message = input<string>();
}

@Component({
  selector: "app-configuration-form",
  imports: [FormsModule, RouterLink, ConfigurationFieldError],
  templateUrl: "./configuration-form.html",
  styleUrl: "./configuration-form.css",
  encapsulation: ViewEncapsulation.None,
})
export class ConfigurationForm {
  readonly name = input.required<string>();
  readonly displayName = input.required<string>();
  readonly nameLabel = input("Name");
  readonly nameField = input("name");
  readonly creating = input.required<boolean>();
  readonly saving = input.required<boolean>();
  readonly errors = input.required<FieldErrors>();
  readonly listLink = input.required<string[]>();
  readonly submitted = output<MetadataPatch | null>();
  private readonly nameElement =
    viewChild.required<ElementRef<HTMLInputElement>>("nameElement");
  nameInput?: string;
  displayNameInput?: string;
  nameError = "";

  focusName(): void {
    this.nameElement().nativeElement.focus();
  }

  submit(): void {
    if (this.saving()) {
      return;
    }
    const name = this.nameInput ?? this.name();
    this.nameError =
      this.creating() && !/^[a-z0-9]+(-[a-z0-9]+)*$/.test(name)
        ? "Use lowercase letters and digits, separated by single hyphens."
        : "";
    if (this.nameError) {
      this.submitted.emit(null);
      return;
    }
    const displayName = this.displayNameInput ?? this.displayName();
    this.submitted.emit({
      ...(this.creating() ? { name } : {}),
      ...(displayName !== this.displayName() ? { displayName } : {}),
    });
  }
}
