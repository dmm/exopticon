import { Component, OnInit, Input } from '@angular/core';

import { Camera } from '../camera';

@Component({
  selector: 'app-camera-overlay',
  templateUrl: './camera-overlay.component.html',
  styleUrls: ['./camera-overlay.component.css']
})
export class CameraOverlayComponent implements OnInit {

  @Input() camera: Camera;

  constructor() { }

  ngOnInit() {
  }

}
