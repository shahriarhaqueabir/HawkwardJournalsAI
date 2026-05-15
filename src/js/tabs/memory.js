const { invoke } = window.__TAURI__.core;

class MemoryManager {
    constructor() {
        this.canvas = document.getElementById('memory-canvas');
        this.ctx = this.canvas.getContext('2d');
        this.nodes = [];
        this.edges = [];
        this.selectedNode = null;
        this.expandedNodes = new Set();
        this.visibleNodes = null;
        this.zoom = 1;
        this.offsetX = 0;
        this.offsetY = 0;
        this.isDragging = false;
        this.didPan = false;
        this.dragNode = null;
        this.hoverNode = null;
        this.lastMouseX = 0;
        this.lastMouseY = 0;
        
        this.currentView = 'map';
        this.currentCategory = '';
        this.facts = [];
        
        this.init();
    }

    async init() {
        this.resize();
        window.addEventListener('resize', () => this.resize());
        this.setupEvents();
        await this.loadData();
        await this.loadFacts();
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
        this.selectedNode = null;
        this.expandedNodes.clear();
        this.visibleNodes = null;
        this.buildNodeAdjacency();
        this.renderNodeDetails();
    }

    buildNodeAdjacency() {
        this.nodes.forEach(node => {
            node.neighbors = new Set();
            node.relations = [];
        });

        this.edges.forEach(edge => {
            edge.source.neighbors.add(edge.target.id);
            edge.target.neighbors.add(edge.source.id);
            edge.source.relations.push(`→ ${edge.label} → ${edge.target.id}`);
            edge.target.relations.push(`← ${edge.label} ← ${edge.source.id}`);
        });
    }

    setupEvents() {
        this.canvas.addEventListener('mousedown', (e) => {
            const { x, y } = this.getMousePos(e);
            this.dragNode = this.findNodeAt(x, y);
            if (!this.dragNode) {
                this.isDragging = true;
                this.didPan = false;
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
                this.didPan = true;
                this.lastMouseX = e.clientX;
                this.lastMouseY = e.clientY;
            }
            
            this.updateTooltip(e);
        });

        window.addEventListener('mouseup', () => {
            this.dragNode = null;
            this.isDragging = false;
        });

        this.canvas.addEventListener('click', (e) => {
            if (this.didPan) return;
            const { x, y } = this.getMousePos(e);
            const node = this.findNodeAt(x, y);
            if (node) {
                this.selectNode(node);
            }
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
        document.getElementById('btn-node-expand').addEventListener('click', () => this.expandSelectedNode());
        document.getElementById('btn-node-collapse').addEventListener('click', () => this.collapseSelectedNode());

        // View Switcher
        document.querySelectorAll('.switcher-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                const view = btn.dataset.view;
                this.switchView(view);
            });
        });

        // Category Filters
        document.querySelectorAll('.cat-btn').forEach(btn => {
            btn.addEventListener('click', () => {
                this.currentCategory = btn.dataset.cat;
                document.querySelectorAll('.cat-btn').forEach(b => b.classList.remove('active'));
                btn.classList.add('active');
                this.renderFacts();
            });
        });
    }

    switchView(view) {
        this.currentView = view;
        document.querySelectorAll('.switcher-btn').forEach(b => {
            b.classList.toggle('active', b.dataset.view === view);
        });

        const mapView = document.getElementById('map-view');
        const bankView = document.getElementById('bank-view');
        const stats = document.querySelector('.memory-stats');

        if (view === 'map') {
            mapView.classList.remove('hidden');
            bankView.classList.add('hidden');
            stats.classList.remove('hidden');
        } else {
            mapView.classList.add('hidden');
            bankView.classList.remove('hidden');
            stats.classList.add('hidden');
            this.loadFacts();
        }
    }

    async loadFacts() {
        try {
            this.facts = await invoke('profile_get_facts', { category: null });
            this.renderFacts();
        } catch (e) {
            console.error('Failed to load facts:', e);
        }
    }

    renderFacts() {
        const container = document.getElementById('fact-list');
        const filtered = this.currentCategory 
            ? this.facts.filter(f => f.category === this.currentCategory)
            : this.facts;

        if (filtered.length === 0) {
            container.innerHTML = `<div class="empty-state">No facts found in this category.</div>`;
            return;
        }

        container.innerHTML = filtered.map(f => `
            <div class="fact-card" data-id="${f.id}">
                <div class="fact-header">
                    <span class="fact-category">${f.category}</span>
                    <span class="fact-confidence">${(f.confidence * 100).toFixed(0)}% confidence</span>
                </div>
                <div class="fact-content">${f.content}</div>
                <div class="fact-footer">
                    <span class="fact-key">${f.fact_key}</span>
                    <button class="btn-delete-fact" onclick="window.memoryManager.deleteFact('${f.id}')" title="Delete Fact">🗑️</button>
                </div>
            </div>
        `).join('');
    }

    async deleteFact(id) {
        if (!confirm('Are you sure you want to delete this fact?')) return;
        try {
            await invoke('profile_delete_fact', { id });
            await this.loadFacts();
        } catch (e) {
            console.error('Failed to delete fact:', e);
        }
    }

    getMousePos(e) {
        const rect = this.canvas.getBoundingClientRect();
        return {
            x: (e.clientX - rect.left - this.offsetX) / this.zoom,
            y: (e.clientY - rect.top - this.offsetY) / this.zoom
        };
    }

    findNodeAt(x, y) {
        const scope = this.getRenderableNodes();
        return scope.find(n => {
            const dx = n.x - x;
            const dy = n.y - y;
            return Math.sqrt(dx*dx + dy*dy) < n.radius * 2;
        });
    }

    getRenderableNodes() {
        if (!this.visibleNodes || this.visibleNodes.size === 0) {
            return this.nodes;
        }
        return this.nodes.filter(n => this.visibleNodes.has(n.id));
    }

    getRenderableEdges() {
        if (!this.visibleNodes || this.visibleNodes.size === 0) {
            return this.edges;
        }
        return this.edges.filter(e => this.visibleNodes.has(e.source.id) && this.visibleNodes.has(e.target.id));
    }

    selectNode(node) {
        this.selectedNode = node;
        if (this.expandedNodes.size === 0) {
            this.expandedNodes.add(node.id);
            this.recomputeVisibleSubgraph();
        }
        this.renderNodeDetails();
    }

    expandSelectedNode() {
        if (!this.selectedNode) return;
        this.expandedNodes.add(this.selectedNode.id);
        this.recomputeVisibleSubgraph();
        this.renderNodeDetails();
    }

    collapseSelectedNode() {
        if (!this.selectedNode) return;
        this.expandedNodes.delete(this.selectedNode.id);
        if (this.expandedNodes.size === 0) {
            this.visibleNodes = null;
        } else {
            this.recomputeVisibleSubgraph();
        }
        this.renderNodeDetails();
    }

    recomputeVisibleSubgraph() {
        if (this.expandedNodes.size === 0) {
            this.visibleNodes = null;
            return;
        }
        const visible = new Set();
        this.nodes.forEach(node => {
            if (this.expandedNodes.has(node.id)) {
                visible.add(node.id);
                node.neighbors.forEach(id => visible.add(id));
            }
        });
        this.visibleNodes = visible;
    }

    renderNodeDetails() {
        const emptyEl = document.getElementById('node-detail-empty');
        const contentEl = document.getElementById('node-detail-content');
        if (!this.selectedNode) {
            emptyEl.classList.remove('hidden');
            contentEl.classList.add('hidden');
            return;
        }

        emptyEl.classList.add('hidden');
        contentEl.classList.remove('hidden');
        document.getElementById('detail-node-name').innerText = this.selectedNode.id;
        document.getElementById('detail-node-degree').innerText = `${this.selectedNode.neighbors.size} direct connection(s)`;

        const relEl = document.getElementById('detail-node-relations');
        const relations = this.selectedNode.relations || [];
        if (relations.length === 0) {
            relEl.innerHTML = '<li>No linked relations yet.</li>';
        } else {
            relEl.innerHTML = relations.slice(0, 25).map(r => `<li>${r}</li>`).join('');
        }

        const expandBtn = document.getElementById('btn-node-expand');
        const collapseBtn = document.getElementById('btn-node-collapse');
        const expanded = this.expandedNodes.has(this.selectedNode.id);
        expandBtn.disabled = expanded;
        collapseBtn.disabled = !expanded;
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
        const renderNodes = this.getRenderableNodes();
        const renderEdges = this.getRenderableEdges();

        for (let i = 0; i < renderNodes.length; i++) {
            for (let j = i + 1; j < renderNodes.length; j++) {
                const n1 = renderNodes[i];
                const n2 = renderNodes[j];
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
        renderEdges.forEach(e => {
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
        renderNodes.forEach(n => {
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
        const renderEdges = this.getRenderableEdges();
        const renderNodes = this.getRenderableNodes();

        renderEdges.forEach(e => {
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
        renderNodes.forEach(n => {
            const isSelected = this.selectedNode && this.selectedNode.id === n.id;
            this.ctx.fillStyle = isSelected ? '#f0b429' : (n === this.hoverNode ? '#5dade2' : n.color);
            this.ctx.beginPath();
            this.ctx.arc(n.x, n.y, isSelected ? n.radius + 2 : n.radius, 0, Math.PI * 2);
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
window.memoryManager = new MemoryManager();
// Global instance
window.MemoryManager = MemoryManager;
