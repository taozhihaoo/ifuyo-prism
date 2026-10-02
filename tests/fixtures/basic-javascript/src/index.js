const { helper } = require("./helper");

class Store {
    constructor() {
        this.items = [];
    }

    add(item) {
        this.items.push(item);
    }
}

function createStore() {
    return new Store();
}

module.exports = { createStore };
