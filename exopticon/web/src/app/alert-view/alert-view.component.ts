import { Router, ActivatedRoute, ParamMap } from '@angular/router';
import { Component, OnInit } from '@angular/core';

@Component({
  selector: 'app-alert-view',
  templateUrl: './alert-view.component.html',
  styleUrls: ['./alert-view.component.css']
})
export class AlertViewComponent implements OnInit {

  public observationId: number;

  constructor(public route: ActivatedRoute) { }

  ngOnInit(): void {
    this.observationId = parseInt(this.route.snapshot.paramMap.get('id'), 10);
  }

}

