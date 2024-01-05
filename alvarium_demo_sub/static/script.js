document.addEventListener('DOMContentLoaded', function() {
    const sensorButtons = document.querySelectorAll('.sensor-btn');
    const readingButtons = document.querySelectorAll('.reading-btn');
    const annotationButtons = document.querySelectorAll('.annotation-btn');

    // Function to render a graph for a sensor
    function renderSensorGraph(canvas, index) {
        let sensorsData = JSON.parse(canvas.getAttribute('data-sensor-readings'));
        console.log("Render sensor graph: ", sensorsData);
        const readingsData = sensorsData.map(reading => [reading.value, reading.id]);
        console.log(readingsData);
        const timestamps = sensorsData.map(reading => new Date(reading.timestamp));
        console.log(timestamps);

        const ctx = document.querySelectorAll('.sensor-graph')[index].getContext('2d');

        new Chart(ctx, {
            type: 'line',
            data: {
                labels: timestamps,
                datasets: [{
                    label: 'Reading Values',
                    data: readingsData.map(reading => reading[0]),
                    borderColor: 'rgba(75, 192, 192, 1)',
                    borderWidth: 1,
                    fill: false,
                }]
            },
            options: {
                plugins: {
                    tooltip: {
                        enabled: true,
                        mode: 'index',
                        intersect: false,
                        callbacks: {
                            label: function (tooltipItem, data) {
                                // You can use tooltipItem and data to determine what to display
                                var label = "Reading";

                                if (label) {
                                    label += ': ';
                                }

                                label += readingsData[tooltipItem.datasetIndex][1].slice(0, 10) || '';

                                label += '    Value: ';
                                // Customizing the label
                                label += readingsData[tooltipItem.datasetIndex][0] || '';
                                // You can also add additional information here
                                return label;
                            }
                        }
                    }
                },
                responsive: true,
                scales: {
                    x: {
                        type: 'time',
                        position: 'bottom',
                        time: {
                            unit: 'minute',
                        },
                        title: {
                            display: true,
                            text: 'Timestamps'
                        }
                    },
                    y: {
                        title: {
                            display: true,
                            text: 'Reading Values'
                        }
                    }
                }
            }
        });
    }

    sensorButtons.forEach((sensorButton, index) => {
        sensorButton.addEventListener('click', () => {
            const readings = sensorButton.parentElement.querySelector('.readings');
            let containerId = '.charts-container-' + sensorButton.id;
            const container = document.querySelector(containerId);

            readings.style.display = readings.style.display === 'block' ? 'none' : 'block';
            container.style.display = readings.style.display === 'block' ? 'flex' : 'none';
            let sensorId = sensorButton.id;
            console.log("Render sensor graph: ", sensorId);
            const graphId = ".graph-container-" + sensorId;
            console.log(graphId);
            const graph = document.querySelector(graphId);
            graph.style.display = readings.style.display;


            // Render the graph when the sensor is expanded
            if (readings.style.display === 'block') {
                // Set the canvas context property to the sensor object
                const canvas = document.querySelectorAll('.sensor-graph')[index];
                renderSensorGraph(canvas, index);
            }
        });
    });

    readingButtons.forEach(readingButton => {
        readingButton.addEventListener('click', () => {
            const annotations = readingButton.parentElement.querySelector('.annotations');
            annotations.style.display = annotations.style.display === 'block' ? 'none' : 'block';
        });
    });

    annotationButtons.forEach(annotationButton => {
        annotationButton.addEventListener('click', () => {
            const annotationDetails = annotationButton.nextElementSibling;
            annotationDetails.style.display = annotationDetails.style.display === 'block' ? 'none' : 'block';
        });
    });

});
