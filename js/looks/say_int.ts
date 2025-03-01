let output_div;

export function say_int(data: number): void {
  if (typeof output_div == 'undefined') {
    output_div = document.getElementById('hq-output');
  }
  output_div.innerHTML += data.toString();
  output_div.innerHTML += '<br>';
}