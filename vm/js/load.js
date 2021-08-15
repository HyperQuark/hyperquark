async function fetchProj(id) {
  const project = await fetch(`https://projects.scratch.mit.edu/${id}/`);
  const json = await project.json();
  return json;
}

async function load(id, memory) {
  const json = await fetchProj(id);
}

export default load;
