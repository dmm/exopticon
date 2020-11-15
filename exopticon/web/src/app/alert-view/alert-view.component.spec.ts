import { ComponentFixture, TestBed, waitForAsync } from "@angular/core/testing";
import { AlertViewComponent } from "./alert-view.component";

describe("AlertViewComponent", () => {
  let component: AlertViewComponent;
  let fixture: ComponentFixture<AlertViewComponent>;

  beforeEach(waitForAsync(() => {
    TestBed.configureTestingModule({
      declarations: [AlertViewComponent],
    }).compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(AlertViewComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });
});
