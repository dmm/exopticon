import { Component, OnInit, Input } from '@angular/core';

@Component({
  selector: 'app-camera-status-overlay',
  templateUrl: './camera-status-overlay.component.html',
  styleUrls: ['./camera-status-overlay.component.css']
})
export class CameraStatusOverlayComponent implements OnInit {

  @Input() status: string;

  constructor() { }

  ngOnInit() {
  }

}
