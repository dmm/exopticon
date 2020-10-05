import { Component } from "@angular/core";
import { ActivatedRoute } from "@angular/router";

@Component({
  selector: "app-root",
  templateUrl: "./app.component.html",
  styleUrls: ["./app.component.css"],
})
export class AppComponent {
  title = "exopticon";
  fullscreen = false;

  constructor(private route: ActivatedRoute) {}

  ngOnInit() {
    this.route.queryParamMap.subscribe((params) => {
      if (params.has("fs")) {
        this.fullscreen = params.get("fs") === "true";
      } else {
        this.fullscreen = false;
      }
    });
  }
}
