import { async, ComponentFixture, TestBed } from "@angular/core/testing";
import { AnalysisPanelComponent } from "./analysis-panel.component";

describe("AnalysisPanelComponent", () => {
  let component: AnalysisPanelComponent;
  let fixture: ComponentFixture<AnalysisPanelComponent>;

  beforeEach(async(() => {
    TestBed.configureTestingModule({
      declarations: [AnalysisPanelComponent],
    }).compileComponents();
  }));

  beforeEach(() => {
    fixture = TestBed.createComponent(AnalysisPanelComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it("should create", () => {
    expect(component).toBeTruthy();
  });
});
