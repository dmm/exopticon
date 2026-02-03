import { ComponentFixture, TestBed } from '@angular/core/testing';

import { CompatibilityCheckComponent } from './compatibility-check.component';

describe('CompatibilityCheckComponent', () => {
  let component: CompatibilityCheckComponent;
  let fixture: ComponentFixture<CompatibilityCheckComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CompatibilityCheckComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(CompatibilityCheckComponent);
    component = fixture.componentInstance;
    await fixture.whenStable();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
