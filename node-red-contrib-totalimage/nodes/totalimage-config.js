/**
 * TotalImage Configuration Node
 *
 * Configures connection to TotalImage MCP server or Fire Marshal
 */

module.exports = function(RED) {
    function TotalImageConfigNode(config) {
        RED.nodes.createNode(this, config);

        this.name = config.name;
        this.host = config.host || 'localhost';
        this.port = config.port || 3002;
        this.protocol = config.protocol || 'http';
        this.timeout = config.timeout || 30000;

        // Build base URL
        this.baseUrl = `${this.protocol}://${this.host}:${this.port}`;

        // Store credentials securely
        if (this.credentials) {
            this.apiKey = this.credentials.apiKey;
        }
    }

    RED.nodes.registerType('totalimage-config', TotalImageConfigNode, {
        credentials: {
            apiKey: { type: 'password' }
        }
    });
};
