import { AsyncPipe } from "@angular/common";
import { Component, OnInit } from "@angular/core";
import { ActivatedRoute, ParamMap, Router } from "@angular/router";
import { forkJoin, Observable } from "rxjs";
import { map, switchMap } from "rxjs/operators";
import { Camera } from "../camera";
import { CameraGroup } from "../camera-group";
import { CameraGroupService } from "../camera-group.service";
import { CameraService } from "../camera.service";

interface CameraGroupDetail {
  group: CameraGroup;
  memberCameras: Camera[];
}

@Component({
  selector: "app-camera-group-detail",
  templateUrl: "./camera-group-detail.component.html",
  styleUrls: ["./camera-group-detail.component.css"],
  imports: [AsyncPipe],
})
export class CameraGroupDetailComponent implements OnInit {
  public detail$!: Observable<CameraGroupDetail>;

  constructor(
    public route: ActivatedRoute,
    public router: Router,
    private cameraService: CameraService,
    private cameraGroupService: CameraGroupService,
  ) {}

  ngOnInit(): void {
    this.detail$ = this.route.paramMap.pipe(
      switchMap((params: ParamMap) => {
        const name = params.get("name") ?? "";
        return forkJoin({
          group: this.cameraGroupService.getCameraGroup(name),
          cameras: this.cameraService.getCameras(),
        });
      }),
      map((res) => {
        const cameraMap = new Map(
          res.cameras.map((camera) => [camera.metadata.name, camera]),
        );
        return {
          group: res.group,
          memberCameras: res.group.spec.members
            .map((name) => cameraMap.get(name))
            .filter((camera): camera is Camera => camera !== undefined),
        };
      }),
    );
  }

  returnToCameraGroups() {
    this.router.navigate(["camera_groups"]);
  }
}
