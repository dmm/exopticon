import { ComponentFixture, TestBed } from "@angular/core/testing";

import { CameraGroupListComponent } from "./camera-group-list.component";

describe("CameraGroupListComponent", () => {
  let component: CameraGroupListComponent;
  let fixture: ComponentFixture<CameraGroupListComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [CameraGroupListComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(CameraGroupListComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });
});
