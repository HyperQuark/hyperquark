function load (id, memory) {
  return new Promise(async (resolve, reject) => {
    const project = await fetch(`https://projects.scratch.mit.edu/${id}/`);
    const json = await project.json();
  });
}

export default load;