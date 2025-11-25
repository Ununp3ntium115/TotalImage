/**
 * TotalImage List Partitions Node
 *
 * Lists all partitions/zones in a disk image file.
 */

const axios = require('axios');

module.exports = function(RED) {
    function TotalImageListPartitionsNode(config) {
        RED.nodes.createNode(this, config);

        const node = this;
        this.server = RED.nodes.getNode(config.server);
        this.imagePath = config.imagePath;
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

            node.status({ fill: 'blue', shape: 'dot', text: 'listing partitions...' });

            try {
                const response = await axios.post(
                    `${node.server.baseUrl}/mcp`,
                    {
                        jsonrpc: '2.0',
                        id: Date.now(),
                        method: 'tools/call',
                        params: {
                            name: 'list_partitions',
                            arguments: {
                                path: imagePath,
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

                // Parse the result content
                let partitions = result;
                if (result.content?.[0]?.text) {
                    try {
                        partitions = JSON.parse(result.content[0].text);
                    } catch (e) {
                        // Use raw result if parsing fails
                    }
                }

                msg.payload = {
                    imagePath: imagePath,
                    partitions: partitions.zones || partitions,
                    partitionTable: partitions.partition_table || 'unknown',
                    count: (partitions.zones || partitions).length || 0,
                    timestamp: new Date().toISOString()
                };

                node.status({ fill: 'green', shape: 'dot', text: `${msg.payload.count} partitions` });
                send(msg);

                // Clear status after 3 seconds
                setTimeout(() => node.status({}), 3000);

            } catch (error) {
                node.status({ fill: 'red', shape: 'ring', text: 'error' });
                node.error(`List partitions failed: ${error.message}`, msg);
                if (done) done(error);
                return;
            }

            if (done) done();
        });
    }

    RED.nodes.registerType('totalimage-list-partitions', TotalImageListPartitionsNode);
};
