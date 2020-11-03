import { TestBed } from '@angular/core/testing';

import { CameraPanelService } from './camera-panel.service';

describe('CameraPanelService', () => {
  let service: CameraPanelService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(CameraPanelService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });
});
