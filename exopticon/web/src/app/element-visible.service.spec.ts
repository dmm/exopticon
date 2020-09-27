import { TestBed } from '@angular/core/testing';

import { ElementVisibleService } from './element-visible.service';

describe('ElementVisibleService', () => {
  let service: ElementVisibleService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(ElementVisibleService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
