import { TestBed } from '@angular/core/testing';
import { NetWorthChartComponent } from './net-worth-chart.component';

describe('NetWorthChartComponent', () => {
  it('draws one polyline point per value, scaled into the view box', () => {
    const fixture = TestBed.createComponent(NetWorthChartComponent);
    fixture.componentRef.setInput('points', [
      { date: '2026-08-31', value: 100 },
      { date: '2026-09-30', value: 300 },
      { date: '2026-10-04', value: 200 },
    ]);
    fixture.detectChanges();
    const polyline = fixture.nativeElement.querySelector('polyline') as SVGPolylineElement;
    expect(polyline.getAttribute('points')).toBe('0,100 150,0 300,50');
  });

  it('renders nothing with fewer than two points', () => {
    const fixture = TestBed.createComponent(NetWorthChartComponent);
    fixture.componentRef.setInput('points', [{ date: '2026-10-04', value: 1 }]);
    fixture.detectChanges();
    expect(fixture.nativeElement.querySelector('polyline')).toBeNull();
  });
});
