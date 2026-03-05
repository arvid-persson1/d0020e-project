async function search() {
    const operator = document.getElementById("operator").value;

    const conditionElements = document.querySelectorAll("#conditions .condition");

    const conditions = [];

    conditionElements.forEach(cond => {
        const field = cond.querySelector(".field").value;
        const value = cond.querySelector(".value").value;

        if (value.trim() !== "") {
            conditions.push({ field, value });
        }
    });

    if (conditions.length === 0) {
        alert("Please enter at least one search condition.");
        return;
    }

    const selectedSources = Array.from(
        document.querySelectorAll("#sources input:checked")
    ).map(cb => cb.value);

    const response = await fetch("http://localhost:3000/query", {
        method: "POST",
        headers: {
            "Content-Type": "application/json"
        },
        body: JSON.stringify({
            operator,
            conditions,
            sources: selectedSources
        })
    });

    const data = await response.json();

    const resultsList = document.getElementById("results");
    resultsList.innerHTML = "";

    if (data.length === 0) {
        resultsList.innerHTML = "<li>No results found</li>";
        return;
    }

    data.forEach(result => {
        const li = document.createElement("li");
        const book = result.item;
        li.textContent = `"${book.title}" by ${book.author} (ISBN: ${book.isbn}) - Source: ${result.source}`;
        resultsList.appendChild(li);
    });

    document.getElementById("resultsCount").innerText =
        `Found ${data.length} books`;
}

function addCondition() {
    const container = document.getElementById("conditions");

    const operator = document.getElementById("operator").value.toUpperCase();

    const div = document.createElement("div");
    div.className = "condition";

    div.innerHTML = `
        <span class="condition-operator">${operator}</span>

        <select class="field">
            <option value="author">Author</option>
            <option value="title">Title</option>
            <option value="isbn">ISBN</option>
        </select>

        <input type="text" class="value" placeholder="Enter value" />
    `;

    container.appendChild(div);
}

async function addSource() {
    const name = document.getElementById("sourceName").value;
    const url = document.getElementById("sourceUrl").value;
    const format = document.getElementById("sourceFormat").value;

    try {
        const response = await fetch("http://localhost:3000/sources", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify({
                name: name,
                url: url,
                format: format
            })
        });

        if (!response.ok) {
            throw new Error("Failed to add source");
        }

        document.getElementById("sourceStatus").innerText =
            "Source added successfully!";
        document.getElementById("sourceName").value = "";
        document.getElementById("sourceUrl").value = "";


    } catch (error) {
        document.getElementById("sourceStatus").innerText =
            "Error: " + error.message;
    }
    await loadSources();
}

document.getElementById("addBookForm").addEventListener("submit", async (e) => {
    e.preventDefault();

    const book = {
        title: document.getElementById("title").value,
        author: document.getElementById("author").value,
        isbn: document.getElementById("isbn").value,
        source: document.getElementById("bookSource").value
    };

    try {
        const response = await fetch("http://localhost:3000/books", {
            method: "POST",
            headers: {
                "Content-Type": "application/json"
            },
            body: JSON.stringify(book)
        });

        const text = await response.text();

        if (!response.ok) {
            throw new Error(text);
        }

        document.getElementById("addResult").innerText = text;

    } catch (error) {
        document.getElementById("addResult").innerText =
            "Error adding book: " + error.message;
    }

    document.getElementById("addBookForm").reset();
});

async function loadSources() {
    const res = await fetch("http://localhost:3000/sources");
    const sources = await res.json();

    const dropdown = document.getElementById("bookSource");
    const searchContainer = document.getElementById("sources");

    dropdown.innerHTML = "";
    searchContainer.innerHTML = "";

    sources.forEach(src => {

        // ----- Add Book dropdown -----
        const option = document.createElement("option");
        option.value = src.name;
        option.textContent = src.name;
        dropdown.appendChild(option);

        // ----- Search source checkboxes -----
        const label = document.createElement("label");

        const checkbox = document.createElement("input");
        checkbox.type = "checkbox";
        checkbox.value = src.name;

        label.appendChild(checkbox);
        label.appendChild(document.createTextNode(" " + src.name));

        searchContainer.appendChild(label);
        searchContainer.appendChild(document.createElement("br"));
    });
}

window.addEventListener("DOMContentLoaded", () => {
    loadSources();
});