/**
 * TotalImage List Files Node
 *
 * Lists files in a disk image filesystem with support for subdirectories.
 */

const axios = require('axios');

module.exports = function(RED) {
    function TotalImageListFilesNode(config) {
        RED.nodes.createNode(this, config);

        const node = this;
        this.server = RED.nodes.getNode(config.server);
        this.imagePath = config.imagePath;
        this.zoneIndex = config.zoneIndex || 0;
        this.directory = config.directory || '';

        node.on('input', async function(msg, send, done) {
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

            node.status({ fill: 'blue', shape: 'dot', text: 'listing...' });

            try {
                const response = await axios.post(
                    `${node.server.baseUrl}/mcp`,
                    {
                        jsonrpc: '2.0',
                        id: Date.now(),
                        method: 'tools/call',
                        params: {
                            name: 'list_files',
                            arguments: {
                                path: imagePath,
                                zone_index: msg.payload?.zoneIndex ?? node.zoneIndex,
                                directory: msg.payload?.directory ?? node.directory
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
                    files: result.content?.[0]?.text ? JSON.parse(result.content[0].text) : result,
                    zoneIndex: msg.payload?.zoneIndex ?? node.zoneIndex,
                    directory: msg.payload?.directory ?? node.directory,
                    timestamp: new Date().toISOString()
                };

                node.status({ fill: 'green', shape: 'dot', text: `${msg.payload.files?.length || 0} files` });
                send(msg);

                setTimeout(() => node.status({}), 3000);

            } catch (error) {
                node.status({ fill: 'red', shape: 'ring', text: 'error' });
                node.error(`List files failed: ${error.message}`, msg);
                if (done) done(error);
                return;
            }

            if (done) done();
        });
    }

    RED.nodes.registerType('totalimage-list-files', TotalImageListFilesNode);
};
