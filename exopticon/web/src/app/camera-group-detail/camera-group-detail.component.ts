import { Component, OnInit } from "@angular/core";
import { FormArray, FormControl, FormGroup } from "@angular/forms";
import { ActivatedRoute, ParamMap, Router } from "@angular/router";
import { forkJoin, Observable, of, Subscription } from "rxjs";
import { switchMap } from "rxjs/operators";
import { Camera, CameraId } from "../camera";
import { ALL_GROUP_ID, CameraGroup } from "../camera-group";
import { CameraGroupService } from "../camera-group.service";
import { CameraService } from "../camera.service";

@Component({
  selector: "app-camera-group-detail",
  templateUrl: "./camera-group-detail.component.html",
  styleUrls: ["./camera-group-detail.component.css"],
  standalone: false,
})
export class CameraGroupDetailComponent implements OnInit {
  public stuff$: Observable<any>;
  public stuffSubscription: Subscription;
  public hideMembership = false;

  // form values
  public groupForm = new FormGroup({
    id: new FormControl(""),
    name: new FormControl(""),
    members: new FormArray([
      new FormGroup({
        id: new FormControl(""),
        name: new FormControl(""),
        include: new FormControl(false),
      }),
    ]),
  });

  constructor(
    public route: ActivatedRoute,
    public router: Router,
    private cameraService: CameraService,
    private cameraGroupService: CameraGroupService,
  ) {}

  ngOnInit(): void {
    this.stuffSubscription = this.route.paramMap
      .pipe(
        switchMap((params: ParamMap) => {
          let cameraGroup$: Observable<CameraGroup>;
          let id = params.get("id");
          if (id !== null) {
            cameraGroup$ = this.cameraGroupService.getCameraGroup(id);
          } else {
            // initialize empty form group
            cameraGroup$ = of(new CameraGroup());

            console.log("Empty form group");
          }

          return forkJoin({
            cameraGroup: cameraGroup$,
            cameras: this.cameraService.getCameras(),
          });
        }),
      )
      .subscribe((res) => {
        this.setFormFromGroup(res.cameraGroup, res.cameras);
      });
  }

  ngOnDestroy(): void {
    this.stuffSubscription.unsubscribe();
  }

  onSubmit() {
    let values = this.groupForm.value;
    let cameraGroup = new CameraGroup();

    cameraGroup.id = values.id;
    cameraGroup.name = values.name;
    cameraGroup.members = values.members
      .filter((m) => m.include)
      .map((m) => m.id);
    this.cameraGroupService
      .setCameraGroup(cameraGroup)
      .toPromise()
      .then((cameraGroup) => {
        this.router.navigate(["camera_groups", cameraGroup.id]);
      })
      .catch((err) => console.log(err));
  }

  deleteCameraGroup(event) {
    this.cameraGroupService
      .deleteCameraGroup(event.id)
      .toPromise()
      .then(() => {
        this.router.navigate(["camera_groups"]);
      });
    2;
  }

  returnToCameraGroups() {
    this.router.navigate(["camera_groups"]);
  }

  setFormFromGroup(group: CameraGroup, cameras: Camera[]) {
    this.hideMembership = group.id === ALL_GROUP_ID;
    let cameraMap: Map<CameraId, Camera> = new Map();
    cameras.filter((c) => c.enabled).forEach((c) => cameraMap.set(c.id, c));
    let memberArray = new FormArray([]);
    // Add included cameras
    group.members.forEach((id) => {
      let c = cameraMap.get(id);
      cameraMap.delete(id);
      memberArray.push(
        new FormGroup({
          id: new FormControl(c.id),
          name: new FormControl(c.name),
          include: new FormControl(true),
        }),
      );
    });
    // Add non-included cameras
    cameraMap.forEach((c) => {
      memberArray.push(
        new FormGroup({
          id: new FormControl(c.id),
          name: new FormControl(c.name),
          include: new FormControl(false),
        }),
      );
    });
    this.groupForm = new FormGroup({
      id: new FormControl(group.id),
      name: new FormControl(group.name),
      members: memberArray,
    });
  }

  get members() {
    return this.groupForm.get("members") as FormArray;
  }

  set members(newMembers: FormArray) {
    let arr = this.groupForm.get("members") as FormArray;

    arr.reset();

    arr = newMembers;
  }

  rotateMembers(k) {
    let m: any[] = this.members.value;

    for (let i = 0; k < i; i++) {
      if (k < 0) {
        m.unshift(m.pop());
      } else {
        m.push(m.shift);
      }
    }

    this.members = new FormArray(m);
  }

  memberUp(event) {
    let i = event.index;
    let m = this.members.value;
    if (i > 0 && m.length > 1) {
      let t = m[i - 1];
      m[i - 1] = m[i];
      m[i] = t;
      this.members.setValue(m);
    }
  }

  memberDown(event) {
    let i = event.index;
    let m = this.members.value;
    if (i < m.length - 1 && m.length > 1) {
      let t = m[i + 1];
      m[i + 1] = m[i];
      m[i] = t;
      this.members.setValue(m);
    }
  }
}
