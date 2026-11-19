import { Component, input, output } from "@angular/core";
import { FormsModule } from "@angular/forms";
import {
  ConfigurationFieldError,
  ConfigurationForm,
  FieldErrors,
  MetadataPatch,
} from "./configuration-form";

export interface ConfigurationUser {
  username: string;
  displayName: string;
}

@Component({
  selector: "app-user-configuration-form",
  imports: [FormsModule, ConfigurationForm, ConfigurationFieldError],
  templateUrl: "./user-form.component.html",
})
export class UserFormComponent {
  readonly user = input.required<ConfigurationUser | null>();
  readonly listLink = input.required<string[]>();
  readonly errors = input.required<FieldErrors>();
  readonly saving = input.required<boolean>();
  readonly saved = output<object>();
  password = "";
  passwordError = "";

  save(metadata: MetadataPatch | null): void {
    this.passwordError = !this.user() && !this.password ? "Enter a value." : "";
    if (!metadata || this.passwordError) {
      return;
    }
    this.saved.emit({
      username: metadata.name,
      displayName: metadata.displayName,
      password: this.password || undefined,
    });
  }
}
