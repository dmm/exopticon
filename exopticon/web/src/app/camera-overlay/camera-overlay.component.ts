import { Component, Input, OnInit } from "@angular/core";
import { Camera } from "../camera";
import { CameraService, PtzDirection } from "../camera.service";

@Component({
  selector: "app-camera-overlay",
  templateUrl: "./camera-overlay.component.html",
  styleUrls: ["./camera-overlay.component.css"],
})
export class CameraOverlayComponent implements OnInit {
  @Input() camera: Camera;

  private directions = PtzDirection;

  constructor(private cameraService: CameraService) {}

  ngOnInit() {}

  ptz(direction: PtzDirection) {
    this.cameraService.ptz(this.camera.id, direction);
  }
}
