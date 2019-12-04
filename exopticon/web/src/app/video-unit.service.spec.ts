import { TestBed } from '@angular/core/testing';

import { VideoUnitService } from './video-unit.service';

describe('VideoUnitService', () => {
  beforeEach(() => TestBed.configureTestingModule({}));

  it('should be created', () => {
    const service: VideoUnitService = TestBed.get(VideoUnitService);
    expect(service).toBeTruthy();
  });
});
