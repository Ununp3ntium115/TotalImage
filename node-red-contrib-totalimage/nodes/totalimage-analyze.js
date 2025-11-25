/**
 * TotalImage Analyze Node
 *
 * Analyzes a disk image file and returns comprehensive information
 * about vault type, partitions, and filesystems.
 */

const axios = require('axios');

module.exports = function(RED) {
    function TotalImageAnalyzeNode(config) {
        RED.nodes.createNode(this, config);

        const node = this;
        this.server = RED.nodes.getNode(config.server);
        this.imagePath = config.imagePath;
        this.deepScan = config.deepScan || false;
        this.useCache = config.useCache !== false;

        node.on('input', async function(msg, send, done) {
            // Use msg.payload.path or configured path
            const imagePath = msg.payload?.path || msg.imagePath || node.imagePath;

            if (!imagePath) {
                node.error('No image path specified', msg);
                if (done) done(new Error('No image path specified'));
                return;
            }

            if (!node.server) {
                node.error('No TotalImage server configured', msg);
                if (done) done(new Error('No server configured'));
                return;
            }

            node.status({ fill: 'blue', shape: 'dot', text: 'analyzing...' });

            try {
                const response = await axios.post(
                    `${node.server.baseUrl}/mcp`,
                    {
                        jsonrpc: '2.0',
                        id: Date.now(),
                        method: 'tools/call',
                        params: {
                            name: 'analyze_disk_image',
                            arguments: {
                                path: imagePath,
                                deep_scan: msg.payload?.deepScan ?? node.deepScan,
                                cache: msg.payload?.useCache ?? node.useCache
                            }
                        }
                    },
                    {
                        timeout: node.server.timeout,
                        headers: node.server.apiKey ? {
                            'Authorization': `Bearer ${node.server.apiKey}`
                        } : {}
                    }
                );

                if (response.data.error) {
                    throw new Error(response.data.error.message);
                }

                const result = response.data.result;

                msg.payload = {
                    imagePath: imagePath,
                    analysis: result,
                    timestamp: new Date().toISOString()
                };

                node.status({ fill: 'green', shape: 'dot', text: 'success' });
                send(msg);

                // Clear status after 3 seconds
                setTimeout(() => node.status({}), 3000);

            } catch (error) {
                node.status({ fill: 'red', shape: 'ring', text: 'error' });
                node.error(`Analysis failed: ${error.message}`, msg);
                if (done) done(error);
                return;
            }

            if (done) done();
        });
    }

    RED.nodes.registerType('totalimage-analyze', TotalImageAnalyzeNode);
};
