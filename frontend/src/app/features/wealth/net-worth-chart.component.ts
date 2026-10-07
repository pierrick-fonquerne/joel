import { Component, computed, input } from '@angular/core';

export interface ChartPoint {
  date: string;
  value: number;
}

const WIDTH = 300;
const HEIGHT = 100;

@Component({
  selector: 'app-net-worth-chart',
  template: `
    @if (polylinePoints(); as line) {
      <svg [attr.viewBox]="'0 0 ' + width + ' ' + height" preserveAspectRatio="none" class="chart" role="img" aria-label="Évolution du patrimoine net">
        <polyline [attr.points]="line" fill="none" stroke="currentColor" stroke-width="2" vector-effect="non-scaling-stroke" />
      </svg>
    }
  `,
  styles: `.chart { width: 100%; height: 8rem; color: #4f8cc9; }`,
})
export class NetWorthChartComponent {
  readonly points = input.required<ChartPoint[]>();
  protected readonly width = WIDTH;
  protected readonly height = HEIGHT;

  protected readonly polylinePoints = computed(() => {
    const points = this.points();
    if (points.length < 2) {
      return null;
    }
    const values = points.map((point) => point.value);
    const minimum = Math.min(...values);
    const range = Math.max(...values) - minimum || 1;
    const step = WIDTH / (points.length - 1);
    return points
      .map((point, index) => `${Math.round(index * step)},${Math.round(HEIGHT - ((point.value - minimum) / range) * HEIGHT)}`)
      .join(' ');
  });
}
