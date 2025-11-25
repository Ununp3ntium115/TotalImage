/**
 * TotalImage Validate Integrity Node
 *
 * Validates the integrity of a disk image file by checking
 * checksums, boot sectors, and partition table structures.
 */

const axios = require('axios');

module.exports = function(RED) {
    function TotalImageValidateIntegrityNode(config) {
        RED.nodes.createNode(this, config);

        const node = this;
        this.server = RED.nodes.getNode(config.server);
        this.imagePath = config.imagePath;
        this.checkChecksums = config.checkChecksums !== false;
        this.checkBootSectors = config.checkBootSectors !== false;

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

            node.status({ fill: 'blue', shape: 'dot', text: 'validating...' });

            try {
                const response = await axios.post(
                    `${node.server.baseUrl}/mcp`,
                    {
                        jsonrpc: '2.0',
                        id: Date.now(),
                        method: 'tools/call',
                        params: {
                            name: 'validate_integrity',
                            arguments: {
                                path: imagePath,
                                check_checksums: msg.payload?.checkChecksums ?? node.checkChecksums,
                                check_boot_sectors: msg.payload?.checkBootSectors ?? node.checkBootSectors
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
                let validation = result;
                if (result.content?.[0]?.text) {
                    try {
                        validation = JSON.parse(result.content[0].text);
                    } catch (e) {
                        // Use raw result if parsing fails
                    }
                }

                const isValid = validation.valid !== false &&
                               (!validation.issues || validation.issues.length === 0);

                msg.payload = {
                    imagePath: imagePath,
                    valid: isValid,
                    validation: validation,
                    issues: validation.issues || [],
                    checksums: validation.checksums || {},
                    timestamp: new Date().toISOString()
                };

                if (isValid) {
                    node.status({ fill: 'green', shape: 'dot', text: 'valid' });
                } else {
                    node.status({ fill: 'yellow', shape: 'ring', text: `${(validation.issues || []).length} issues` });
                }

                send(msg);

                // Clear status after 3 seconds
                setTimeout(() => node.status({}), 3000);

            } catch (error) {
                node.status({ fill: 'red', shape: 'ring', text: 'error' });
                node.error(`Validation failed: ${error.message}`, msg);
                if (done) done(error);
                return;
            }

            if (done) done();
        });
    }

    RED.nodes.registerType('totalimage-validate-integrity', TotalImageValidateIntegrityNode);
};
