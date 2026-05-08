const { invoke } = window.__TAURI__.core;

class MemoryMap {
    constructor() {
        this.canvas = document.getElementById('memory-canvas');
        this.ctx = this.canvas.getContext('2d');
        this.nodes = [];
        this.edges = [];
        this.zoom = 1;
        this.offsetX = 0;
        this.offsetY = 0;
        this.isDragging = false;
        this.dragNode = null;
        this.hoverNode = null;
        this.lastMouseX = 0;
        this.lastMouseY = 0;
        
        this.init();
    }

    async init() {
        this.resize();
        window.addEventListener('resize', () => this.resize());
        this.setupEvents();
        await this.loadData();
        this.animate();
    }

    resize() {
        const container = this.canvas.parentElement;
        this.canvas.width = container.clientWidth;
        this.canvas.height = container.clientHeight;
        this.offsetX = this.canvas.width / 2;
        this.offsetY = this.canvas.height / 2;
    }

    async loadData() {
        try {
            // Query all nodes and edges from GraphQLite
            // We'll use a Cypher query to get everything
            const data = await invoke('graph_query', { 
                cypher: "MATCH (n)-[r]->(m) RETURN n.name as source, type(r) as rel, m.name as target" 
            });

            const rawData = Array.isArray(data) ? data : [];
            this.processData(rawData);
            
            document.getElementById('node-count').innerText = `${this.nodes.length} nodes`;
            document.getElementById('edge-count').innerText = `${this.edges.length} edges`;
        } catch (e) {
            console.error('Failed to load memory data:', e);
        }
    }

    processData(data) {
        const nodeMap = new Map();
        const newEdges = [];

        data.forEach(item => {
            if (!nodeMap.has(item.source)) {
                nodeMap.set(item.source, { 
                    id: item.source, 
                    x: Math.random() * 400 - 200, 
                    y: Math.random() * 400 - 200,
                    vx: 0, vy: 0,
                    radius: 6,
                    color: '#3498db'
                });
            }
            if (!nodeMap.has(item.target)) {
                nodeMap.set(item.target, { 
                    id: item.target, 
                    x: Math.random() * 400 - 200, 
                    y: Math.random() * 400 - 200,
                    vx: 0, vy: 0,
                    radius: 6,
                    color: '#3498db'
                });
            }
            newEdges.push({
                source: nodeMap.get(item.source),
                target: nodeMap.get(item.target),
                label: item.rel
            });
        });

        this.nodes = Array.from(nodeMap.values());
        this.edges = newEdges;
    }

    setupEvents() {
        this.canvas.addEventListener('mousedown', (e) => {
            const { x, y } = this.getMousePos(e);
            this.dragNode = this.findNodeAt(x, y);
            if (!this.dragNode) {
                this.isDragging = true;
                this.lastMouseX = e.clientX;
                this.lastMouseY = e.clientY;
            }
        });

        window.addEventListener('mousemove', (e) => {
            const { x, y } = this.getMousePos(e);
            this.hoverNode = this.findNodeAt(x, y);

            if (this.dragNode) {
                this.dragNode.x = x;
                this.dragNode.y = y;
            } else if (this.isDragging) {
                this.offsetX += (e.clientX - this.lastMouseX);
                this.offsetY += (e.clientY - this.lastMouseY);
                this.lastMouseX = e.clientX;
                this.lastMouseY = e.clientY;
            }
            
            this.updateTooltip(e);
        });

        window.addEventListener('mouseup', () => {
            this.dragNode = null;
            this.isDragging = false;
        });

        this.canvas.addEventListener('wheel', (e) => {
            e.preventDefault();
            const scale = e.deltaY > 0 ? 0.9 : 1.1;
            this.zoom *= scale;
        });

        document.getElementById('refresh-memory').addEventListener('click', () => this.loadData());
        document.getElementById('zoom-in').addEventListener('click', () => this.zoom *= 1.2);
        document.getElementById('zoom-out').addEventListener('click', () => this.zoom *= 0.8);
        document.getElementById('reset-view').addEventListener('click', () => {
            this.zoom = 1;
            this.offsetX = this.canvas.width / 2;
            this.offsetY = this.canvas.height / 2;
        });
    }

    getMousePos(e) {
        const rect = this.canvas.getBoundingClientRect();
        return {
            x: (e.clientX - rect.left - this.offsetX) / this.zoom,
            y: (e.clientY - rect.top - this.offsetY) / this.zoom
        };
    }

    findNodeAt(x, y) {
        return this.nodes.find(n => {
            const dx = n.x - x;
            const dy = n.y - y;
            return Math.sqrt(dx*dx + dy*dy) < n.radius * 2;
        });
    }

    updateTooltip(e) {
        const tooltip = document.getElementById('memory-tooltip');
        if (this.hoverNode) {
            tooltip.style.display = 'block';
            tooltip.style.left = `${e.clientX + 15}px`;
            tooltip.style.top = `${e.clientY + 15}px`;
            tooltip.innerText = this.hoverNode.id;
        } else {
            tooltip.style.display = 'none';
        }
    }

    updatePhysics() {
        const k = 0.05; // spring constant
        const repulsion = 1000;
        
        // Repulsion
        for (let i = 0; i < this.nodes.length; i++) {
            for (let j = i + 1; j < this.nodes.length; j++) {
                const n1 = this.nodes[i];
                const n2 = this.nodes[j];
                const dx = n1.x - n2.x;
                const dy = n1.y - n2.y;
                const distSq = dx*dx + dy*dy || 1;
                const force = repulsion / distSq;
                const fx = (dx / Math.sqrt(distSq)) * force;
                const fy = (dy / Math.sqrt(distSq)) * force;
                
                n1.vx += fx; n1.vy += fy;
                n2.vx -= fx; n2.vy -= fy;
            }
        }

        // Attraction (edges)
        this.edges.forEach(e => {
            const dx = e.target.x - e.source.x;
            const dy = e.target.y - e.source.y;
            const dist = Math.sqrt(dx*dx + dy*dy) || 1;
            const force = (dist - 100) * k;
            const fx = (dx / dist) * force;
            const fy = (dy / dist) * force;
            
            e.source.vx += fx; e.source.vy += fy;
            e.target.vx -= fx; e.target.vy -= fy;
        });

        // Apply and dampen
        this.nodes.forEach(n => {
            if (n === this.dragNode) return;
            n.x += n.vx;
            n.y += n.vy;
            n.vx *= 0.9;
            n.vy *= 0.9;
            
            // Central gravity
            n.vx -= n.x * 0.005;
            n.vy -= n.y * 0.005;
        });
    }

    animate() {
        this.updatePhysics();
        this.draw();
        requestAnimationFrame(() => this.animate());
    }

    draw() {
        this.ctx.clearRect(0, 0, this.canvas.width, this.canvas.height);
        this.ctx.save();
        this.ctx.translate(this.offsetX, this.offsetY);
        this.ctx.scale(this.zoom, this.zoom);

        // Draw Edges
        this.ctx.strokeStyle = 'rgba(255, 255, 255, 0.15)';
        this.ctx.lineWidth = 1 / this.zoom;
        this.edges.forEach(e => {
            this.ctx.beginPath();
            this.ctx.moveTo(e.source.x, e.source.y);
            this.ctx.lineTo(e.target.x, e.target.y);
            this.ctx.stroke();
            
            if (this.zoom > 0.8) {
                this.ctx.fillStyle = 'rgba(255, 255, 255, 0.4)';
                this.ctx.font = `${8 / this.zoom}px Inter`;
                this.ctx.fillText(e.label, (e.source.x + e.target.x) / 2, (e.source.y + e.target.y) / 2);
            }
        });

        // Draw Nodes
        this.nodes.forEach(n => {
            this.ctx.fillStyle = n === this.hoverNode ? '#5dade2' : n.color;
            this.ctx.beginPath();
            this.ctx.arc(n.x, n.y, n.radius, 0, Math.PI * 2);
            this.ctx.fill();
            
            if (this.zoom > 0.5) {
                this.ctx.fillStyle = 'white';
                this.ctx.font = `${10 / this.zoom}px Inter`;
                this.ctx.textAlign = 'center';
                this.ctx.fillText(n.id, n.x, n.y + n.radius + 12 / this.zoom);
            }
        });

        this.ctx.restore();
    }
}

// Global instance
window.MemoryMap = MemoryMap;
