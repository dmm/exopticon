import { ComponentFixture, TestBed } from "@angular/core/testing";

import { CameraGroupDetailComponent } from "./camera-group-detail.component";

describe("CameraGroupDetailComponent", () => {
  let component: CameraGroupDetailComponent;
  let fixture: ComponentFixture<CameraGroupDetailComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      declarations: [CameraGroupDetailComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(CameraGroupDetailComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });
});
