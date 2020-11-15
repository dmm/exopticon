import { ComponentFixture, TestBed, waitForAsync } from "@angular/core/testing";
import { CameraPanelComponent } from "./camera-panel.component";

describe("CameraPanelComponent", () => {
  let component: CameraPanelComponent;
  let fixture: ComponentFixture<CameraPanelComponent>;

  beforeEach(waitForAsync(() => {
    TestBed.configureTestingModule({
      declarations: [CameraPanelComponent],
    }).compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(CameraPanelComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });
});
