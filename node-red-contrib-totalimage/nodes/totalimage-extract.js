/**
 * TotalImage Extract Node
 *
 * Extracts files from a disk image to a destination path.
 */

const axios = require('axios');

module.exports = function(RED) {
    function TotalImageExtractNode(config) {
        RED.nodes.createNode(this, config);

        const node = this;
        this.server = RED.nodes.getNode(config.server);
        this.imagePath = config.imagePath;
        this.filePath = config.filePath;
        this.outputPath = config.outputPath;
        this.zoneIndex = config.zoneIndex || 0;

        node.on('input', async function(msg, send, done) {
            const imagePath = msg.payload?.imagePath || msg.imagePath || node.imagePath;
            const filePath = msg.payload?.filePath || msg.filePath || node.filePath;
            const outputPath = msg.payload?.outputPath || msg.outputPath || node.outputPath;

            if (!imagePath || !filePath || !outputPath) {
                node.error('Missing required paths (imagePath, filePath, outputPath)', msg);
                if (done) done(new Error('Missing required paths'));
                return;
            }

            if (!node.server) {
                node.error('No TotalImage server configured', msg);
                if (done) done(new Error('No server configured'));
                return;
            }

            node.status({ fill: 'blue', shape: 'dot', text: 'extracting...' });

            try {
                const response = await axios.post(
                    `${node.server.baseUrl}/mcp`,
                    {
                        jsonrpc: '2.0',
                        id: Date.now(),
                        method: 'tools/call',
                        params: {
                            name: 'extract_file',
                            arguments: {
                                image_path: imagePath,
                                file_path: filePath,
                                output_path: outputPath,
                                zone_index: msg.payload?.zoneIndex ?? node.zoneIndex
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
                    filePath: filePath,
                    outputPath: outputPath,
                    result: result,
                    success: true,
                    timestamp: new Date().toISOString()
                };

                node.status({ fill: 'green', shape: 'dot', text: 'extracted' });
                send(msg);

                setTimeout(() => node.status({}), 3000);

            } catch (error) {
                node.status({ fill: 'red', shape: 'ring', text: 'error' });
                node.error(`Extract failed: ${error.message}`, msg);
                if (done) done(error);
                return;
            }

            if (done) done();
        });
    }

    RED.nodes.registerType('totalimage-extract', TotalImageExtractNode);
};
