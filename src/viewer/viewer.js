// Shared JS for both viewers. Each viewer also emits a small inline <script>
// containing data-dependent plot calls or shape-data assignments; that script
// runs *before* this one, so any globals it defines are visible here.

// Used by viewer_raw's per-shape Plotly.newPlot calls.
var common = { margin: { t: 10, r: 10, b: 30, l: 45 }, showlegend: false };

// Used by viewer_structured's shape-popup hover behavior. The inline data
// script populates this map with sample arrays keyed by shape id.
window.shapes = window.shapes || {};

document.addEventListener('DOMContentLoaded', function () {
  // Shape-popup hover (viewer_structured). No-op in viewer_raw because there
  // are no .shape-link elements there.
  var layout = {
    margin: { t: 10, r: 10, b: 30, l: 40 },
    showlegend: false,
    width: 480,
    height: 240,
    xaxis: { title: 'sample' }
  };
  var config = { responsive: false, displaylogo: false, displayModeBar: false };
  var registry = document.getElementById('shape-registry');

  function ensurePlotted(plotDiv, shapeId) {
    if (!plotDiv.dataset.rendered) {
      Plotly.newPlot(
        plotDiv,
        [{ y: window.shapes[shapeId], mode: 'lines', line: { width: 1.2 } }],
        layout,
        config
      );
      plotDiv.dataset.rendered = '1';
    }
  }

  document.querySelectorAll('.shape-link').forEach(function (link) {
    var shapeId = link.dataset.shape;
    var plotDiv = document.getElementById('shape-plot-' + shapeId);
    var popup = link.querySelector('.shape-popup');
    if (!plotDiv || !popup) return;
    link.addEventListener('mouseenter', function () {
      popup.appendChild(plotDiv);
      ensurePlotted(plotDiv, shapeId);
    });
    link.addEventListener('mouseleave', function () {
      if (registry) registry.appendChild(plotDiv);
    });
  });
});
